import AppKit
import Carbon

/// Manages global keyboard shortcuts for the status bar app
final class HotkeyManager {
    typealias HotkeyHandler = () -> Void

    private var hotkeyRef: EventHotKeyRef?
    private var handler: HotkeyHandler?
    private var eventHandler: EventHandlerRef?

    // Hotkey signature - unique identifier for this app's hotkeys
    private let hotkeySignature: FourCharCode = {
        let chars = "AGNB" // "Again Bar"
        var code: FourCharCode = 0
        for char in chars.utf8 {
            code = (code << 8) | FourCharCode(char)
        }
        return code
    }()

    private let hotkeyID: UInt32 = 1

    init() {}

    deinit {
        unregister()
    }

    /// Register a global hotkey (default: Cmd+Shift+A)
    /// - Parameters:
    ///   - keyCode: The virtual key code (default: 0 for 'A')
    ///   - modifiers: Modifier flags (default: Cmd+Shift)
    ///   - handler: Callback when hotkey is pressed
    /// - Returns: true if registration succeeded
    @discardableResult
    func register(
        keyCode: UInt32 = UInt32(kVK_ANSI_A),
        modifiers: UInt32 = UInt32(cmdKey | shiftKey),
        handler: @escaping HotkeyHandler
    ) -> Bool {
        self.handler = handler

        // Set up event type spec for hotkey events
        var eventType = EventTypeSpec(
            eventClass: OSType(kEventClassKeyboard),
            eventKind: UInt32(kEventHotKeyPressed)
        )

        // Install event handler
        let status = InstallEventHandler(
            GetApplicationEventTarget(),
            { (_, event, userData) -> OSStatus in
                guard let userData = userData else { return OSStatus(eventNotHandledErr) }
                let manager = Unmanaged<HotkeyManager>.fromOpaque(userData).takeUnretainedValue()

                var hotkeyID = EventHotKeyID()
                let err = GetEventParameter(
                    event,
                    EventParamName(kEventParamDirectObject),
                    EventParamType(typeEventHotKeyID),
                    nil,
                    MemoryLayout<EventHotKeyID>.size,
                    nil,
                    &hotkeyID
                )

                if err == noErr && hotkeyID.id == manager.hotkeyID {
                    DispatchQueue.main.async {
                        manager.handler?()
                    }
                }

                return noErr
            },
            1,
            &eventType,
            Unmanaged.passUnretained(self).toOpaque(),
            &eventHandler
        )

        guard status == noErr else {
            NSLog("HotkeyManager: Failed to install event handler: \(status)")
            return false
        }

        // Register the hotkey
        let hotkeyIDStruct = EventHotKeyID(signature: hotkeySignature, id: hotkeyID)
        let registerStatus = RegisterEventHotKey(
            keyCode,
            modifiers,
            hotkeyIDStruct,
            GetApplicationEventTarget(),
            0,
            &hotkeyRef
        )

        if registerStatus != noErr {
            NSLog("HotkeyManager: Failed to register hotkey: \(registerStatus)")
            return false
        }

        NSLog("HotkeyManager: Registered global hotkey Cmd+Shift+A")
        return true
    }

    /// Unregister the global hotkey
    func unregister() {
        if let hotkeyRef = hotkeyRef {
            UnregisterEventHotKey(hotkeyRef)
            self.hotkeyRef = nil
            NSLog("HotkeyManager: Unregistered hotkey")
        }
        if let eventHandler = eventHandler {
            RemoveEventHandler(eventHandler)
            self.eventHandler = nil
        }
        handler = nil
    }
}
