#!/bin/bash
GUID="$1"
if [ -z "$GUID" ]; then
    exit 1
fi
osascript <<EOF
tell application "iTerm2"
    activate
    repeat with w in windows
        tell w
            repeat with t in tabs
                tell t
                    repeat with s in sessions
                        if id of s is "$GUID" then
                            select
                        end if
                    end repeat
                end tell
            end repeat
        end tell
    end repeat
end tell
EOF
