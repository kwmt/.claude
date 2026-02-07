on open location thisURL
    -- URL format: x-claude-iterm://switch?guid=XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX
    set AppleScript's text item delimiters to "guid="
    set guidParts to text items of thisURL
    if (count of guidParts) > 1 then
        set guid to item 2 of guidParts

        tell application "iTerm2"
            activate
            repeat with w in windows
                tell w
                    repeat with t in tabs
                        tell t
                            repeat with s in sessions
                                if id of s is guid then
                                    select
                                end if
                            end repeat
                        end tell
                    end repeat
                end tell
            end repeat
        end tell
    end if

    tell me to quit
end open location

on idle
    quit
    return 1
end idle
