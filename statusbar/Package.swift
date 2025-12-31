// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "AgainStatusBar",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .executable(
            name: "again-statusbar",
            targets: ["AgainStatusBar"]
        )
    ],
    targets: [
        .executableTarget(
            name: "AgainStatusBar",
            path: "Sources"
        )
    ]
)
