# @tairseach/client

Node.js client library for **Tairseach** — the macOS system integration daemon that provides access to Contacts, Calendar, Reminders, Location, and Screen Capture via a local Unix socket.

## Installation

```bash
npm install @tairseach/client
```

## Quick Start

```typescript
import { createClient } from '@tairseach/client';

// Connect to the daemon
const client = await createClient();

// Search contacts
const contacts = await client.searchContacts({ query: 'John' });
console.log(contacts);

// Get calendar events for the next week
const events = await client.getCalendarEvents({
  startDate: new Date().toISOString(),
  endDate: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString(),
});

// Disconnect when done
client.disconnect();
```

## Connection

The client connects to the Tairseach daemon via Unix socket at `~/.tairseach/tairseach.sock`.

```typescript
import { TairseachClient } from '@tairseach/client';

// Manual connection management
const client = new TairseachClient({
  socketPath: '/path/to/custom.sock', // Optional: custom socket path
  timeout: 30000,                      // Request timeout in ms
  reconnect: true,                     // Auto-reconnect on disconnect
  reconnectInterval: 1000,             // Time between reconnect attempts
  maxReconnectAttempts: 5,             // Max reconnection tries
});

await client.connect();

// Check connection status
if (client.isConnected()) {
  // ...
}

// Listen for connection events
client.on('connected', () => console.log('Connected!'));
client.on('disconnected', () => console.log('Disconnected'));
client.on('reconnecting', (e) => console.log(`Reconnecting (attempt ${e.data.attempt})`));
client.on('error', (e) => console.error('Error:', e.data));
```

## API Reference

### Contacts

```typescript
// List all contacts
const { contacts, total } = await client.listContacts(limit?, offset?);

// Search contacts
const results = await client.searchContacts({ 
  query: 'search term',
  limit: 10 
});

// Get a single contact
const contact = await client.getContact('contact-id');

// Create a contact
const newContact = await client.createContact({
  firstName: 'John',
  lastName: 'Doe',
  emails: [{ label: 'work', value: 'john@example.com' }],
  phones: [{ label: 'mobile', value: '+1-555-0123' }],
});

// Update a contact
const updated = await client.updateContact({
  id: 'contact-id',
  jobTitle: 'Senior Developer',
});

// Delete a contact
await client.deleteContact('contact-id');
```

### Permissions

Tairseach requires macOS permissions to access system data. Use these methods to check and request permissions:

```typescript
// Check a specific permission
const info = await client.checkPermission('contacts');
// Returns: { type: 'contacts', status: 'authorized', canRequest: false }

// List all permission statuses
const permissions = await client.listPermissions();

// Request a permission (prompts user if not determined)
const result = await client.requestPermission('calendar');
```

Permission types: `'contacts'`, `'calendar'`, `'reminders'`, `'location'`, `'screen'`

Permission statuses: `'authorized'`, `'denied'`, `'notDetermined'`, `'restricted'`, `'limited'`

### Calendar

```typescript
// List all calendars
const calendars = await client.listCalendars();

// Get events in a date range
const events = await client.getCalendarEvents({
  startDate: '2024-01-01T00:00:00Z',
  endDate: '2024-01-31T23:59:59Z',
  calendarIds: ['calendar-id'], // Optional: filter by calendars
});

// Get a single event
const event = await client.getCalendarEvent('event-id');

// Create an event
const newEvent = await client.createEvent({
  calendarId: 'calendar-id',
  title: 'Team Meeting',
  startDate: '2024-01-15T10:00:00Z',
  endDate: '2024-01-15T11:00:00Z',
  location: 'Conference Room A',
  notes: 'Weekly sync',
  alarms: [{ type: 'display', triggerMinutes: -15 }],
});

// Update an event
const updated = await client.updateEvent({
  id: 'event-id',
  title: 'Updated Meeting Title',
});

// Delete an event
await client.deleteEvent('event-id');
```

### Reminders

```typescript
// List reminder lists
const lists = await client.listReminderLists();

// Get reminders
const reminders = await client.getReminders({
  listIds: ['list-id'],        // Optional: filter by lists
  includeCompleted: false,      // Include completed reminders
  dueBefore: '2024-01-31',     // Filter by due date
  dueAfter: '2024-01-01',
});

// Get a single reminder
const reminder = await client.getReminder('reminder-id');

// Create a reminder
const newReminder = await client.createReminder({
  listId: 'list-id',
  title: 'Buy groceries',
  dueDate: '2024-01-15T17:00:00Z',
  priority: 1, // 0=none, 1=high, 5=medium, 9=low
  notes: 'Milk, eggs, bread',
});

// Update a reminder
const updated = await client.updateReminder({
  id: 'reminder-id',
  title: 'Updated title',
});

// Mark as complete/incomplete
await client.completeReminder('reminder-id');
await client.uncompleteReminder('reminder-id');

// Delete a reminder
await client.deleteReminder('reminder-id');
```

### Location

```typescript
// Get current location
const location = await client.getCurrentLocation({
  desiredAccuracy: 'best', // 'best', 'nearestTenMeters', 'hundredMeters', 'kilometer', 'threeKilometers'
  timeout: 10000,
});

console.log(`Lat: ${location.latitude}, Lng: ${location.longitude}`);
console.log(`Accuracy: ${location.horizontalAccuracy}m`);
```

### Screen Capture

```typescript
// Capture the screen
const capture = await client.captureScreen({
  format: 'png',           // 'png' or 'jpeg'
  quality: 85,             // JPEG quality (0-100)
  displayId: 0,            // Which display to capture
  region: {                // Optional: capture a region
    x: 0,
    y: 0,
    width: 800,
    height: 600,
  },
});

// capture.imageData is base64-encoded
const buffer = Buffer.from(capture.imageData, 'base64');
```

## Error Handling

```typescript
import { TairseachError, ConnectionError, TimeoutError } from '@tairseach/client';

try {
  const contact = await client.getContact('invalid-id');
} catch (error) {
  if (error instanceof ConnectionError) {
    console.error('Connection lost:', error.message);
  } else if (error instanceof TimeoutError) {
    console.error('Request timed out');
  } else if (error instanceof TairseachError) {
    console.error(`RPC Error (${error.code}): ${error.message}`);
  }
}
```

## TypeScript

Full TypeScript support is included. All types are exported:

```typescript
import type {
  Contact,
  ContactEmail,
  ContactPhone,
  Calendar,
  CalendarEvent,
  Reminder,
  ReminderList,
  Location,
  PermissionType,
  PermissionStatus,
  PermissionInfo,
  ScreenCaptureResult,
  TairseachClientOptions,
} from '@tairseach/client';
```

## Raw JSON-RPC

For advanced use cases, you can make raw JSON-RPC calls:

```typescript
const result = await client.call<MyResultType>('namespace.method', {
  param1: 'value1',
  param2: 123,
});
```

## License

MIT
