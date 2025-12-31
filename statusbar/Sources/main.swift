import AppKit

// Main entry point for the status bar app
// SPM executables need explicit NSApplication setup
let app = NSApplication.shared
let delegate = AppDelegate()
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
