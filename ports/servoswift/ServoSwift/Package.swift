// swift-tools-version: 5.9
// The Swift Package Manager package definition for ServoSwift

import PackageDescription

let package = Package(
    name: "ServoSwift",
    platforms: [
        .macOS(.v11)
    ],
    products: [
        .library(
            name: "ServoSwift",
            targets: ["ServoSwift"]
        ),
    ],
    targets: [
        .target(
            name: "ServoSwift",
            dependencies: [],
            path: "Sources",
            publicHeadersPath: "include",
            cSettings: [
                .headerSearchPath("include"),
                .define("SERVO_SWIFT_MACOS")
            ],
            linkerSettings: [
                .linkedLibrary("servoswift", .when(platforms: [.macOS])),
                .linkedFramework("AppKit"),
                .linkedFramework("Metal"),
                .linkedFramework("OpenGL"),
                .linkedFramework("CoreGraphics"),
                .unsafeFlags(["-L../../../target/release"])
            ]
        ),
        .executableTarget(
            name: "ServoSwiftExample",
            dependencies: ["ServoSwift"],
            path: "Examples/macOS"
        )
    ]
)
