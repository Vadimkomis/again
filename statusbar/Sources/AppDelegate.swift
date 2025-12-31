import AppKit

final class AppDelegate: NSObject, NSApplicationDelegate {
    private var statusController: StatusBarController?

    func applicationDidFinishLaunching(_ notification: Notification) {
        #if DEBUG
        NSLog("AgainStatusBar launched")
        #endif
        self.statusController = StatusBarController()
    }

    func applicationWillTerminate(_ notification: Notification) {
        self.statusController?.invalidate()
    }
}
