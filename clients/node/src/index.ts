/**
 * Tairseach Client
 * Node.js client library for the Tairseach macOS system integration daemon
 */

import { Socket, createConnection } from 'node:net';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { EventEmitter } from 'node:events';

import type {
  JsonRpcRequest,
  JsonRpcResponse,
  JsonRpcError,
  TairseachClientOptions,
  TairseachEventType,
  // Contacts
  Contact,
  ContactSearchOptions,
  ContactCreateParams,
  ContactUpdateParams,
  ContactListResult,
  // Permissions
  PermissionType,
  PermissionInfo,
  // Calendar
  Calendar,
  CalendarEvent,
  CalendarEventCreateParams,
  CalendarEventUpdateParams,
  CalendarEventListParams,
  // Reminders
  ReminderList,
  Reminder,
  ReminderCreateParams,
  ReminderUpdateParams,
  ReminderListParams,
  // Location
  Location,
  LocationOptions,
  // Screen
  ScreenCaptureResult,
  ScreenCaptureOptions,
} from './types.js';

// Re-export all types
export * from './types.js';

// ============================================================================
// Error Classes
// ============================================================================

export class TairseachError extends Error {
  constructor(
    message: string,
    public code: number = -1,
    public data?: unknown
  ) {
    super(message);
    this.name = 'TairseachError';
  }

  static fromJsonRpcError(error: JsonRpcError): TairseachError {
    return new TairseachError(error.message, error.code, error.data);
  }
}

export class ConnectionError extends TairseachError {
  constructor(message: string) {
    super(message, -32000);
    this.name = 'ConnectionError';
  }
}

export class TimeoutError extends TairseachError {
  constructor(message: string = 'Request timed out') {
    super(message, -32001);
    this.name = 'TimeoutError';
  }
}

// ============================================================================
// Main Client Class
// ============================================================================

interface PendingRequest {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
  timer: NodeJS.Timeout;
}

const DEFAULT_OPTIONS: Required<TairseachClientOptions> = {
  socketPath: join(homedir(), '.tairseach', 'tairseach.sock'),
  timeout: 30000,
  reconnect: true,
  reconnectInterval: 1000,
  maxReconnectAttempts: 5,
};

export class TairseachClient extends EventEmitter {
  private options: Required<TairseachClientOptions>;
  private socket: Socket | null = null;
  private requestId = 0;
  private pendingRequests = new Map<number | string, PendingRequest>();
  private buffer = '';
  private connected = false;
  private reconnectAttempts = 0;
  private reconnectTimer: NodeJS.Timeout | null = null;

  constructor(options: TairseachClientOptions = {}) {
    super();
    this.options = { ...DEFAULT_OPTIONS, ...options };
  }

  // ==========================================================================
  // Connection Management
  // ==========================================================================

  /**
   * Connect to the Tairseach daemon
   */
  async connect(): Promise<void> {
    if (this.connected) return;

    return new Promise((resolve, reject) => {
      this.socket = createConnection(this.options.socketPath);

      this.socket.on('connect', () => {
        this.connected = true;
        this.reconnectAttempts = 0;
        this.emitEvent('connected');
        resolve();
      });

      this.socket.on('data', (data) => this.handleData(data));

      this.socket.on('error', (error) => {
        if (!this.connected) {
          reject(new ConnectionError(`Failed to connect: ${error.message}`));
        } else {
          this.emitEvent('error', error);
        }
      });

      this.socket.on('close', () => {
        const wasConnected = this.connected;
        this.connected = false;
        this.socket = null;
        
        // Reject all pending requests
        for (const [id, pending] of this.pendingRequests) {
          clearTimeout(pending.timer);
          pending.reject(new ConnectionError('Connection closed'));
          this.pendingRequests.delete(id);
        }

        if (wasConnected) {
          this.emitEvent('disconnected');
          this.maybeReconnect();
        }
      });
    });
  }

  /**
   * Disconnect from the Tairseach daemon
   */
  disconnect(): void {
    this.options.reconnect = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.socket) {
      this.socket.destroy();
      this.socket = null;
    }
    this.connected = false;
  }

  /**
   * Check if client is connected
   */
  isConnected(): boolean {
    return this.connected;
  }

  private maybeReconnect(): void {
    if (!this.options.reconnect) return;
    if (this.reconnectAttempts >= this.options.maxReconnectAttempts) {
      this.emitEvent('error', new Error('Max reconnection attempts reached'));
      return;
    }

    this.reconnectAttempts++;
    this.emitEvent('reconnecting', { attempt: this.reconnectAttempts });

    this.reconnectTimer = setTimeout(() => {
      this.connect().catch(() => {
        // Will trigger close event and retry
      });
    }, this.options.reconnectInterval);
  }

  private emitEvent(type: TairseachEventType, data?: unknown): void {
    this.emit(type, { type, timestamp: new Date(), data });
  }

  // ==========================================================================
  // JSON-RPC Transport
  // ==========================================================================

  private handleData(data: Buffer): void {
    this.buffer += data.toString('utf8');
    
    // Process complete JSON-RPC messages (newline-delimited)
    let newlineIndex: number;
    while ((newlineIndex = this.buffer.indexOf('\n')) !== -1) {
      const line = this.buffer.slice(0, newlineIndex).trim();
      this.buffer = this.buffer.slice(newlineIndex + 1);
      
      if (line) {
        try {
          const response = JSON.parse(line) as JsonRpcResponse;
          this.handleResponse(response);
        } catch {
          // Malformed JSON, ignore
        }
      }
    }
  }

  private handleResponse(response: JsonRpcResponse): void {
    if (response.id == null) return; // Notification, ignore for now

    const pending = this.pendingRequests.get(response.id);
    if (!pending) return;

    clearTimeout(pending.timer);
    this.pendingRequests.delete(response.id);

    if (response.error) {
      pending.reject(TairseachError.fromJsonRpcError(response.error));
    } else {
      pending.resolve(response.result);
    }
  }

  /**
   * Send a raw JSON-RPC request
   */
  async call<T = unknown, P extends object = object>(method: string, params?: P): Promise<T> {
    if (!this.connected || !this.socket) {
      throw new ConnectionError('Not connected to Tairseach daemon');
    }

    const id = ++this.requestId;
    const request: JsonRpcRequest = {
      jsonrpc: '2.0',
      id,
      method,
      params,
    };

    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new TimeoutError());
      }, this.options.timeout);

      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
        timer,
      });

      this.socket!.write(JSON.stringify(request) + '\n');
    });
  }

  // ==========================================================================
  // Contacts API
  // ==========================================================================

  /**
   * List all contacts
   */
  async listContacts(limit?: number, offset?: number): Promise<ContactListResult> {
    return this.call<ContactListResult>('contacts.list', { limit, offset });
  }

  /**
   * Search contacts by name, email, or phone
   */
  async searchContacts(options: ContactSearchOptions): Promise<Contact[]> {
    return this.call<Contact[]>('contacts.search', options);
  }

  /**
   * Get a single contact by ID
   */
  async getContact(id: string): Promise<Contact> {
    return this.call<Contact>('contacts.get', { id });
  }

  /**
   * Create a new contact
   */
  async createContact(params: ContactCreateParams): Promise<Contact> {
    return this.call<Contact>('contacts.create', params);
  }

  /**
   * Update an existing contact
   */
  async updateContact(params: ContactUpdateParams): Promise<Contact> {
    return this.call<Contact>('contacts.update', params);
  }

  /**
   * Delete a contact
   */
  async deleteContact(id: string): Promise<void> {
    return this.call<void>('contacts.delete', { id });
  }

  // ==========================================================================
  // Permissions API
  // ==========================================================================

  /**
   * Check the status of a specific permission
   */
  async checkPermission(type: PermissionType): Promise<PermissionInfo> {
    return this.call<PermissionInfo>('permissions.check', { type });
  }

  /**
   * List all permission statuses
   */
  async listPermissions(): Promise<PermissionInfo[]> {
    return this.call<PermissionInfo[]>('permissions.list');
  }

  /**
   * Request a permission (will prompt user if not determined)
   */
  async requestPermission(type: PermissionType): Promise<PermissionInfo> {
    return this.call<PermissionInfo>('permissions.request', { type });
  }

  // ==========================================================================
  // Calendar API
  // ==========================================================================

  /**
   * List all calendars
   */
  async listCalendars(): Promise<Calendar[]> {
    return this.call<Calendar[]>('calendar.listCalendars');
  }

  /**
   * Get calendar events within a date range
   */
  async getCalendarEvents(params: CalendarEventListParams): Promise<CalendarEvent[]> {
    return this.call<CalendarEvent[]>('calendar.getEvents', params);
  }

  /**
   * Get a single calendar event by ID
   */
  async getCalendarEvent(id: string): Promise<CalendarEvent> {
    return this.call<CalendarEvent>('calendar.getEvent', { id });
  }

  /**
   * Create a new calendar event
   */
  async createEvent(params: CalendarEventCreateParams): Promise<CalendarEvent> {
    return this.call<CalendarEvent>('calendar.createEvent', params);
  }

  /**
   * Update an existing calendar event
   */
  async updateEvent(params: CalendarEventUpdateParams): Promise<CalendarEvent> {
    return this.call<CalendarEvent>('calendar.updateEvent', params);
  }

  /**
   * Delete a calendar event
   */
  async deleteEvent(id: string): Promise<void> {
    return this.call<void>('calendar.deleteEvent', { id });
  }

  // ==========================================================================
  // Reminders API
  // ==========================================================================

  /**
   * List all reminder lists
   */
  async listReminderLists(): Promise<ReminderList[]> {
    return this.call<ReminderList[]>('reminders.listLists');
  }

  /**
   * Get reminders, optionally filtered
   */
  async getReminders(params?: ReminderListParams): Promise<Reminder[]> {
    return this.call<Reminder[]>('reminders.list', params);
  }

  /**
   * Get a single reminder by ID
   */
  async getReminder(id: string): Promise<Reminder> {
    return this.call<Reminder>('reminders.get', { id });
  }

  /**
   * Create a new reminder
   */
  async createReminder(params: ReminderCreateParams): Promise<Reminder> {
    return this.call<Reminder>('reminders.create', params);
  }

  /**
   * Update an existing reminder
   */
  async updateReminder(params: ReminderUpdateParams): Promise<Reminder> {
    return this.call<Reminder>('reminders.update', params);
  }

  /**
   * Mark a reminder as completed
   */
  async completeReminder(id: string): Promise<Reminder> {
    return this.call<Reminder>('reminders.complete', { id });
  }

  /**
   * Mark a reminder as incomplete
   */
  async uncompleteReminder(id: string): Promise<Reminder> {
    return this.call<Reminder>('reminders.uncomplete', { id });
  }

  /**
   * Delete a reminder
   */
  async deleteReminder(id: string): Promise<void> {
    return this.call<void>('reminders.delete', { id });
  }

  // ==========================================================================
  // Location API
  // ==========================================================================

  /**
   * Get the current location
   */
  async getCurrentLocation(options?: LocationOptions): Promise<Location> {
    return this.call<Location>('location.getCurrent', options);
  }

  // ==========================================================================
  // Screen Capture API
  // ==========================================================================

  /**
   * Capture a screenshot
   */
  async captureScreen(options?: ScreenCaptureOptions): Promise<ScreenCaptureResult> {
    return this.call<ScreenCaptureResult>('screen.capture', options);
  }
}

// ============================================================================
// Factory Function
// ============================================================================

/**
 * Create and connect a Tairseach client
 */
export async function createClient(options?: TairseachClientOptions): Promise<TairseachClient> {
  const client = new TairseachClient(options);
  await client.connect();
  return client;
}

// Default export
export default TairseachClient;
