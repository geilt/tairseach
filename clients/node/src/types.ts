/**
 * Tairseach Client Types
 * JSON-RPC 2.0 + Domain-specific types
 */

// ============================================================================
// JSON-RPC 2.0 Types
// ============================================================================

export interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: number | string;
  method: string;
  params?: object;
}

export interface JsonRpcResponse<T = unknown> {
  jsonrpc: '2.0';
  id: number | string | null;
  result?: T;
  error?: JsonRpcError;
}

export interface JsonRpcError {
  code: number;
  message: string;
  data?: unknown;
}

// ============================================================================
// Contact Types
// ============================================================================

export interface Contact {
  id: string;
  firstName?: string;
  lastName?: string;
  fullName?: string;
  nickname?: string;
  organization?: string;
  jobTitle?: string;
  emails?: ContactEmail[];
  phones?: ContactPhone[];
  addresses?: ContactAddress[];
  birthday?: string;
  note?: string;
  imageData?: string;
  createdAt?: string;
  updatedAt?: string;
}

export interface ContactEmail {
  label?: string;
  value: string;
}

export interface ContactPhone {
  label?: string;
  value: string;
}

export interface ContactAddress {
  label?: string;
  street?: string;
  city?: string;
  state?: string;
  postalCode?: string;
  country?: string;
}

export interface ContactSearchOptions {
  query: string;
  limit?: number;
}

export interface ContactCreateParams {
  firstName?: string;
  lastName?: string;
  nickname?: string;
  organization?: string;
  jobTitle?: string;
  emails?: ContactEmail[];
  phones?: ContactPhone[];
  addresses?: ContactAddress[];
  birthday?: string;
  note?: string;
}

export interface ContactUpdateParams extends Partial<ContactCreateParams> {
  id: string;
}

export interface ContactListResult {
  contacts: Contact[];
  total: number;
}

// ============================================================================
// Permission Types
// ============================================================================

export type PermissionType = 
  | 'contacts'
  | 'calendar'
  | 'reminders'
  | 'location'
  | 'screen';

export type PermissionStatus = 
  | 'authorized'
  | 'denied'
  | 'notDetermined'
  | 'restricted'
  | 'limited';

export interface PermissionInfo {
  type: PermissionType;
  status: PermissionStatus;
  canRequest: boolean;
}

// ============================================================================
// Calendar Types
// ============================================================================

export interface Calendar {
  id: string;
  title: string;
  type: 'local' | 'caldav' | 'exchange' | 'subscription' | 'birthday';
  color?: string;
  isEditable: boolean;
  source?: string;
}

export interface CalendarEvent {
  id: string;
  calendarId: string;
  title: string;
  startDate: string;
  endDate: string;
  isAllDay: boolean;
  location?: string;
  notes?: string;
  url?: string;
  attendees?: EventAttendee[];
  recurrenceRule?: string;
  alarms?: EventAlarm[];
}

export interface EventAttendee {
  name?: string;
  email?: string;
  status: 'pending' | 'accepted' | 'declined' | 'tentative';
  isOrganizer: boolean;
}

export interface EventAlarm {
  type: 'display' | 'audio' | 'email';
  triggerMinutes: number;
}

export interface CalendarEventCreateParams {
  calendarId: string;
  title: string;
  startDate: string;
  endDate: string;
  isAllDay?: boolean;
  location?: string;
  notes?: string;
  url?: string;
  alarms?: EventAlarm[];
}

export interface CalendarEventUpdateParams extends Partial<Omit<CalendarEventCreateParams, 'calendarId'>> {
  id: string;
}

export interface CalendarEventListParams {
  calendarIds?: string[];
  startDate: string;
  endDate: string;
}

// ============================================================================
// Reminder Types
// ============================================================================

export interface ReminderList {
  id: string;
  title: string;
  color?: string;
  isEditable: boolean;
}

export interface Reminder {
  id: string;
  listId: string;
  title: string;
  notes?: string;
  dueDate?: string;
  priority: 0 | 1 | 5 | 9; // 0=none, 1=high, 5=medium, 9=low
  isCompleted: boolean;
  completedDate?: string;
  url?: string;
  location?: ReminderLocation;
}

export interface ReminderLocation {
  title?: string;
  latitude?: number;
  longitude?: number;
  radius?: number;
  proximity: 'enter' | 'leave';
}

export interface ReminderCreateParams {
  listId: string;
  title: string;
  notes?: string;
  dueDate?: string;
  priority?: 0 | 1 | 5 | 9;
  url?: string;
  location?: ReminderLocation;
}

export interface ReminderUpdateParams extends Partial<Omit<ReminderCreateParams, 'listId'>> {
  id: string;
}

export interface ReminderListParams {
  listIds?: string[];
  includeCompleted?: boolean;
  dueBefore?: string;
  dueAfter?: string;
}

// ============================================================================
// Location Types
// ============================================================================

export interface Location {
  latitude: number;
  longitude: number;
  altitude?: number;
  horizontalAccuracy?: number;
  verticalAccuracy?: number;
  speed?: number;
  course?: number;
  timestamp: string;
}

export interface LocationOptions {
  desiredAccuracy?: 'best' | 'nearestTenMeters' | 'hundredMeters' | 'kilometer' | 'threeKilometers';
  timeout?: number;
}

// ============================================================================
// Screen Capture Types
// ============================================================================

export interface ScreenCaptureResult {
  imageData: string; // Base64 encoded
  width: number;
  height: number;
  format: 'png' | 'jpeg';
  timestamp: string;
}

export interface ScreenCaptureOptions {
  displayId?: number;
  format?: 'png' | 'jpeg';
  quality?: number; // 0-100 for jpeg
  region?: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
}

// ============================================================================
// Client Options
// ============================================================================

export interface TairseachClientOptions {
  socketPath?: string;
  timeout?: number;
  reconnect?: boolean;
  reconnectInterval?: number;
  maxReconnectAttempts?: number;
}

// ============================================================================
// Event Types
// ============================================================================

export type TairseachEventType = 
  | 'connected'
  | 'disconnected'
  | 'error'
  | 'reconnecting';

export interface TairseachEvent {
  type: TairseachEventType;
  timestamp: Date;
  data?: unknown;
}
