// swift-tools-version: 5.7
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "MatrixRustSDK",
    platforms: [
        .iOS(.v16),
        .macOS(.v12)
    ],
    products: [
        .library(name: "MatrixRustSDK",
                 targets: ["MatrixRustSDK"]),
    ],
    targets: [
        .target(name: "MatrixRustSDK",
                path: "generated/swift",
                swiftSettings: [
                    .unsafeFlags(["-I", "./generated/client_matrix_ffi"])
                ]),
        .testTarget(name: "MatrixRustSDKTests",
                    dependencies: ["MatrixRustSDK"],
                    swiftSettings: [
                        .unsafeFlags(["-I", "./generated/client_matrix_ffi"])
                    ],
                    linkerSettings: [
                        .linkedLibrary("client_matrix_ffi", .when(platforms: [.macOS])),
                        .linkedLibrary("client_matrix_ffiFFI", .when(platforms: [.linux])),
                        .unsafeFlags(["-L./generated/client_matrix_ffi"])
                    ])
    ]
)
