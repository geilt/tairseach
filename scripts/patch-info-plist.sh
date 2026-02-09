#!/bin/bash
# Patch Info.plist with privacy usage descriptions after build

APP_PATH="$1"
if [ -z "$APP_PATH" ]; then
    APP_PATH="src-tauri/target/release/bundle/macos/Tairseach.app"
fi

PLIST="$APP_PATH/Contents/Info.plist"

if [ ! -f "$PLIST" ]; then
    echo "❌ Info.plist not found at $PLIST"
    exit 1
fi

echo "📝 Patching Info.plist with privacy descriptions..."

# Add privacy usage descriptions
/usr/libexec/PlistBuddy -c "Add :NSContactsUsageDescription string 'Tairseach needs access to Contacts to help OpenClaw agents manage your address book.'" "$PLIST" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Set :NSContactsUsageDescription 'Tairseach needs access to Contacts to help OpenClaw agents manage your address book.'" "$PLIST"

/usr/libexec/PlistBuddy -c "Add :NSCalendarsUsageDescription string 'Tairseach needs access to Calendar to help OpenClaw agents manage your schedule.'" "$PLIST" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Set :NSCalendarsUsageDescription 'Tairseach needs access to Calendar to help OpenClaw agents manage your schedule.'" "$PLIST"

/usr/libexec/PlistBuddy -c "Add :NSRemindersUsageDescription string 'Tairseach needs access to Reminders to help OpenClaw agents manage your tasks.'" "$PLIST" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Set :NSRemindersUsageDescription 'Tairseach needs access to Reminders to help OpenClaw agents manage your tasks.'" "$PLIST"

/usr/libexec/PlistBuddy -c "Add :NSAppleEventsUsageDescription string 'Tairseach needs Automation access to control other applications on behalf of OpenClaw agents.'" "$PLIST" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Set :NSAppleEventsUsageDescription 'Tairseach needs Automation access to control other applications on behalf of OpenClaw agents.'" "$PLIST"

echo "✅ Info.plist patched"
echo ""
echo "Current privacy keys:"
/usr/libexec/PlistBuddy -c "Print :NSContactsUsageDescription" "$PLIST" 2>/dev/null && echo "  ✓ NSContactsUsageDescription"
/usr/libexec/PlistBuddy -c "Print :NSCalendarsUsageDescription" "$PLIST" 2>/dev/null && echo "  ✓ NSCalendarsUsageDescription"
