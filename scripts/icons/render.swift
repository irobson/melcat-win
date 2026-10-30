// Renders MelCat's Windows icon assets from the macOS app's masters.
// Runs on macOS (CoreGraphics), where the masters live:
//
//   swift scripts/icons/render.swift <MelCat repo>/Resources/Icons <out dir>
//
// Writes:
//   app-<size>.png for 16, 20, 24, 32, 40, 48, 64, 128, 256 (the .exe icon,
//     packed by pack_ico.py), from AppIcon.png;
//   tray-{active,paused}-{dark,light}.png, 64 px, from the MenuBarIcon.png
//     template glyph: white for a dark taskbar, black for a light one; the
//     paused glyph is the same at 45 % opacity, like the macOS app's dimmed
//     menu bar icon.

import AppKit
import CoreGraphics

let args = CommandLine.arguments
guard args.count == 3 else {
    FileHandle.standardError.write("usage: render.swift <icons dir> <out dir>\n".data(using: .utf8)!)
    exit(2)
}
let icons = URL(fileURLWithPath: args[1])
let out = URL(fileURLWithPath: args[2])
try? FileManager.default.createDirectory(at: out, withIntermediateDirectories: true)
let sRGB = CGColorSpace(name: CGColorSpace.sRGB)!

func load(_ name: String) -> CGImage {
    let image = NSImage(contentsOf: icons.appendingPathComponent(name))!
    var rect = CGRect(origin: .zero, size: image.size)
    return image.cgImage(forProposedRect: &rect, context: nil, hints: nil)!
}

func context(_ size: Int) -> CGContext {
    let ctx = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: size * 4,
                        space: sRGB, bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
    ctx.interpolationQuality = .high
    return ctx
}

func save(_ ctx: CGContext, _ name: String) {
    let rep = NSBitmapImageRep(cgImage: ctx.makeImage()!)
    try! rep.representation(using: .png, properties: [:])!.write(to: out.appendingPathComponent(name))
    print("✓ \(name)")
}

let app = load("AppIcon.png")
for size in [16, 20, 24, 32, 40, 48, 64, 128, 256] {
    let ctx = context(size)
    ctx.draw(app, in: CGRect(x: 0, y: 0, width: size, height: size))
    save(ctx, "app-\(size).png")
}

let glyph = load("MenuBarIcon.png")
for (state, alpha) in [("active", 1.0), ("paused", 0.45)] {
    for (theme, gray) in [("dark", 1.0), ("light", 0.0)] {
        let ctx = context(64)
        let rect = CGRect(x: 0, y: 0, width: 64, height: 64)
        ctx.clip(to: rect, mask: glyph)
        ctx.setFillColor(CGColor(gray: gray, alpha: alpha))
        ctx.fill(rect)
        save(ctx, "tray-\(state)-\(theme).png")
    }
}
