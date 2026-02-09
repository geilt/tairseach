# Frontend Performance Patterns (Tairseach UI)

## Core rules applied

- Keep route transitions ultra-short (`45ms`, opacity only).
- Keep layout stable during data loading (fixed-height containers + skeleton rows).
- Isolate expensive cards with CSS contain:
  - `contain: layout style paint;`
- Use `shallowRef` for large, append-heavy datasets (activity feed).
- Debounce filtering/search input at `150ms`.
- Virtualize long lists with a fixed row height + overscan window.
- Keep auto-refresh light (polling every 1.5s; avoid full app invalidation).

## Virtual list pattern (used in Activity Feed)

1. Keep all records in `shallowRef<ActivityEntry[]>([])`
2. Track `scrollTop` and container height
3. Compute:
   - `startIndex = floor(scrollTop / rowHeight) - overscan`
   - `endIndex = startIndex + visibleCount`
4. Render only `entries.slice(startIndex, endIndex)`
5. Offset with translated spacer to preserve scroll position

## Auto-scroll pattern

- Auto-scroll only when user is already near bottom.
- Pause auto-scroll on hover.
- Resume when user leaves and list updates.

## Dashboard pattern

- Single `Promise.all` fetch for status cards to reduce sequential stalls.
- Render lightweight skeleton rows while loading recent activity.
- Keep cards isolated with contain to prevent cross-card reflow.

## Notes

- Backend log source is currently `~/.tairseach/logs/proxy.log` via Tauri invoke.
- Optional event listener hook exists (`tairseach://activity`) for push updates when backend emits events.
