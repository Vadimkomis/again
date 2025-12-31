import AppKit

final class StatusBarController: NSObject {
    private static let statusIcon: NSImage = {
        if let symbol = NSImage(
            systemSymbolName: "chart.bar.doc.horizontal",
            accessibilityDescription: "Again usage summary")
        {
            symbol.isTemplate = true
            symbol.size = NSSize(width: 18, height: 18)
            return symbol
        }
        let size = NSSize(width: 18, height: 18)
        let image = NSImage(size: size)
        image.lockFocus()
        NSColor.labelColor.setStroke()
        let rect = NSRect(origin: .zero, size: size).insetBy(dx: 3, dy: 3)
        let path = NSBezierPath(roundedRect: rect, xRadius: 4, yRadius: 4)
        path.lineWidth = 1.8
        path.stroke()
        image.unlockFocus()
        image.isTemplate = true
        return image
    }()

    private struct MenuData {
        let title: String
        let entries: [MenuEntry]
    }

    private enum MenuEntry {
        case separator
        case item(MenuLine)
    }

    private struct MenuLine {
        let title: String
        let attributes: [String: String]
    }

    private struct BashCommand {
        let executable: String
        let arguments: [String]
        let refreshAfterRun: Bool
    }

    private enum MenuAction {
        case openURL(URL)
        case runCommand(BashCommand)
    }

    private let statusItem: NSStatusItem
    private var refreshTimer: Timer?
    private let backgroundQueue = DispatchQueue(label: "AgainStatusBar.refresh", qos: .utility)
    private let refreshInterval: TimeInterval = 60 * 5
    private let hotkeyManager = HotkeyManager()

    override init() {
        self.statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        super.init()
        if let button = self.statusItem.button {
            button.image = Self.statusIcon
            button.imagePosition = .imageOnly
            button.imageScaling = .scaleProportionallyDown
            button.title = ""
            button.toolTip = "Again (⌘⇧A)"
        }
        self.statusItem.menu = NSMenu()
        self.refreshMenu()
        self.refreshTimer = Timer.scheduledTimer(
            withTimeInterval: self.refreshInterval,
            repeats: true,
            block: { [weak self] _ in
                self?.refreshMenu()
            })
        self.registerHotkey()
    }

    func invalidate() {
        self.refreshTimer?.invalidate()
        self.refreshTimer = nil
        self.hotkeyManager.unregister()
    }

    private func registerHotkey() {
        hotkeyManager.register { [weak self] in
            self?.showMenu()
        }
    }

    private func showMenu() {
        guard let button = statusItem.button else { return }
        // Programmatically trigger the button click to show the menu
        button.performClick(nil)
    }

    private func refreshMenu() {
        self.showLoadingState()
        self.fetchStatusLines { [weak self] result in
            guard let self else { return }
            switch result {
            case let .success(lines):
                if let data = self.parseMenuData(lines: lines) {
                    self.apply(menuData: data)
                } else {
                    self.showError(message: "Could not parse status output.")
                }
            case let .failure(error):
                self.showError(message: error.localizedDescription)
            }
        }
    }

    private func showLoadingState() {
        let menu = NSMenu()
        let item = NSMenuItem(title: "Loading…", action: nil, keyEquivalent: "")
        item.isEnabled = false
        menu.addItem(item)
        self.statusItem.menu = menu
        self.statusItem.button?.toolTip = "Again: Loading…"
    }

    private func showError(message: String) {
        NSLog("AgainStatusBar error: %@", message)
        let menu = NSMenu()
        let errorItem = NSMenuItem(title: "Again: error", action: nil, keyEquivalent: "")
        errorItem.isEnabled = false
        menu.addItem(errorItem)
        let detailItem = NSMenuItem(title: message, action: nil, keyEquivalent: "")
        detailItem.isEnabled = false
        menu.addItem(detailItem)
        menu.addItem(.separator())
        menu.addItem(self.makeQuitItem())
        self.statusItem.button?.toolTip = "Again: \(message)"
        self.statusItem.menu = menu
    }

    private func apply(menuData: MenuData) {
        let menu = NSMenu()
        for entry in menuData.entries {
            switch entry {
            case .separator:
                menu.addItem(.separator())
            case let .item(line):
                menu.addItem(self.makeMenuItem(line))
            }
        }
        menu.addItem(.separator())
        menu.addItem(self.makeQuitItem())
        self.statusItem.button?.toolTip = menuData.title
        self.statusItem.menu = menu
    }

    private func makeQuitItem() -> NSMenuItem {
        let item = NSMenuItem(title: "Quit Again Status Bar", action: #selector(self.quitApp), keyEquivalent: "q")
        item.target = self
        return item
    }

    @objc private func quitApp() {
        NSApp.terminate(nil)
    }

    private func fetchStatusLines(completion: @escaping (Result<[String], Error>) -> Void) {
        NSLog("AgainStatusBar: starting refresh")
        self.backgroundQueue.async {
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
            process.arguments = self.commandArguments()
            process.environment = self.makeProcessEnvironment()

            let outputPipe = Pipe()
            let errorPipe = Pipe()
            process.standardOutput = outputPipe
            process.standardError = errorPipe

            do {
                try process.run()
                NSLog("AgainStatusBar: launched process %@", process.arguments?.joined(separator: " ") ?? "")
            } catch {
                NSLog("AgainStatusBar: failed to start process %@", error.localizedDescription)
                DispatchQueue.main.async {
                    completion(.failure(error))
                }
                return
            }
            process.waitUntilExit()
            NSLog("AgainStatusBar: process exited \(process.terminationStatus)")
            let data = outputPipe.fileHandleForReading.readDataToEndOfFile()
            let stderrData = errorPipe.fileHandleForReading.readDataToEndOfFile()

            if process.terminationStatus != 0 {
                let message = stderrData.isEmpty
                    ? "again exited with status \(process.terminationStatus)"
                    : String(decoding: stderrData, as: UTF8.self)
                NSLog("AgainStatusBar: non-zero exit: %@", message)
                DispatchQueue.main.async {
                    completion(.failure(NSError(domain: "AgainStatusBar", code: Int(process.terminationStatus), userInfo: [NSLocalizedDescriptionKey: message])))
                }
                return
            }

            let text = String(decoding: data, as: UTF8.self)
            let lines = text
                .components(separatedBy: .newlines)
                .map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
                .filter { !$0.isEmpty }
            DispatchQueue.main.async {
                completion(.success(lines))
            }
        }
    }

    private func commandArguments() -> [String] {
        var args = ["again"]
        args.append(contentsOf: ["--since", "week", "--min", "2", "-n", "15", "--status-bar"])
        return args
    }

    private func parseMenuData(lines: [String]) -> MenuData? {
        guard let first = lines.first else { return nil }
        var entries: [MenuEntry] = []
        for line in lines.dropFirst() {
            if line == "---" {
                entries.append(.separator)
                continue
            }
            entries.append(.item(self.parseMenuLine(line)))
        }
        return MenuData(title: first, entries: entries)
    }

    private func parseMenuLine(_ line: String) -> MenuLine {
        guard let separatorIndex = line.firstIndex(of: "|") else {
            return MenuLine(title: line, attributes: [:])
        }
        let title = line[..<separatorIndex].trimmingCharacters(in: .whitespaces)
        let attributesString = line[line.index(after: separatorIndex)...].trimmingCharacters(in: .whitespaces)
        return MenuLine(title: title, attributes: self.parseAttributes(attributesString))
    }

    private func parseAttributes(_ raw: String) -> [String: String] {
        var attributes: [String: String] = [:]
        let tokens = self.tokenizeAttributes(raw)
        for token in tokens {
            guard let separatorIndex = token.firstIndex(of: "=") else { continue }
            let key = String(token[..<separatorIndex])
            let valuePart = token[token.index(after: separatorIndex)...]
            let value = valuePart
                .replacingOccurrences(of: "\\\"", with: "\"")
                .replacingOccurrences(of: "\\\\", with: "\\")
            attributes[key] = value
        }
        return attributes
    }

    private func tokenizeAttributes(_ raw: String) -> [String] {
        var tokens: [String] = []
        var current = ""
        var inQuotes = false
        var escapeNext = false

        for char in raw {
            if escapeNext {
                current.append(char)
                escapeNext = false
                continue
            }
            if char == "\\" {
                escapeNext = true
                continue
            }
            if char == "\"" {
                inQuotes.toggle()
                continue
            }
            if char == " " && !inQuotes {
                if !current.isEmpty {
                    tokens.append(current)
                    current.removeAll(keepingCapacity: true)
                }
            } else {
                current.append(char)
            }
        }

        if !current.isEmpty {
            tokens.append(current)
        }

        return tokens
    }

    private func makeMenuItem(_ line: MenuLine) -> NSMenuItem {
        let item = NSMenuItem(title: line.title, action: nil, keyEquivalent: "")
        if let colorHex = line.attributes["color"], let color = NSColor(hex: colorHex) {
            let attributed = NSAttributedString(
                string: line.title,
                attributes: [.foregroundColor: color])
            item.attributedTitle = attributed
        }

        if let action = self.action(for: line.attributes) {
            item.target = self
            item.action = action.selector
            item.representedObject = action.payload
            item.isEnabled = true
        } else {
            item.isEnabled = false
        }

        return item
    }

    private func action(for attributes: [String: String]) -> (selector: Selector, payload: Any)? {
        if let href = attributes["href"], let url = URL(string: href) {
            return (#selector(self.openURLFromMenu(_:)), url)
        }
        if let executable = attributes["bash"] {
            let args = attributes
                .filter { $0.key.hasPrefix("param") }
                .sorted { $0.key < $1.key }
                .map { $0.value }
            let refresh = attributes["refresh"] == "true"
            let command = BashCommand(executable: executable, arguments: args, refreshAfterRun: refresh)
            return (#selector(self.runCommandFromMenu(_:)), command)
        }
        return nil
    }

    @objc private func openURLFromMenu(_ sender: NSMenuItem) {
        guard let url = sender.representedObject as? URL else { return }
        NSWorkspace.shared.open(url)
    }

    @objc private func runCommandFromMenu(_ sender: NSMenuItem) {
        guard let action = sender.representedObject as? BashCommand else { return }
        self.backgroundQueue.async { [weak self] in
            guard let self else { return }
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
            process.arguments = [action.executable] + action.arguments
            process.environment = self.makeProcessEnvironment()
            do {
                try process.run()
            } catch {
                DispatchQueue.main.async {
                    self.showError(message: "Failed to run \(action.executable)")
                }
                return
            }
            process.waitUntilExit()
            if action.refreshAfterRun {
                DispatchQueue.main.async {
                    self.refreshMenu()
                }
            }
        }
    }

    private func makeProcessEnvironment() -> [String: String] {
        var environment = ProcessInfo.processInfo.environment
        let cargoBin = (NSHomeDirectory() as NSString).appendingPathComponent(".cargo/bin")
        if let existingPath = environment["PATH"] {
            environment["PATH"] = "\(cargoBin):\(existingPath)"
        } else {
            environment["PATH"] = cargoBin
        }
        return environment
    }
}

private extension NSColor {
    convenience init?(hex: String) {
        var trimmed = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.hasPrefix("#") {
            trimmed.removeFirst()
        }
        guard trimmed.count == 6, let value = Int(trimmed, radix: 16) else {
            return nil
        }
        let red = CGFloat((value >> 16) & 0xFF) / 255.0
        let green = CGFloat((value >> 8) & 0xFF) / 255.0
        let blue = CGFloat(value & 0xFF) / 255.0
        self.init(srgbRed: red, green: green, blue: blue, alpha: 1.0)
    }
}
