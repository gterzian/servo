// swift-tools-version: 6.0
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
            publicHeadersPath: "ServoSwift/include",
            cSettings: [
                .headerSearchPath("ServoSwift/include"),
                .define("SERVO_SWIFT_MACOS")
            ],
            linkerSettings: [
                .linkedLibrary("servoswift", .when(platforms: [.macOS])),
                .linkedFramework("AppKit"),
                .linkedFramework("Metal"),
                .linkedFramework("OpenGL"),
                .linkedFramework("CoreGraphics"),
                .unsafeFlags([
                    "-L/Users/Gregory/Projects/servo/target/debug",
                    "-Xlinker", "-rpath", "-Xlinker", "/Users/Gregory/Projects/servo/target/debug"
                ])
            ]
        ),
        .executableTarget(
            name: "ServoSwiftExample",
            dependencies: ["ServoSwift"],
            path: "Examples/macOS",
            swiftSettings: [
                .swiftLanguageMode(.v6)
            ]
        )
    ]
)
