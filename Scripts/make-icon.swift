import AppKit
import Foundation

// Draws the Yap "listening pill" app icon (Graphite) to a 1024×1024 PNG.
// Usage: swift Scripts/make-icon.swift [outPath]

let outPath = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "build/AppIcon-1024.png"
let S: CGFloat = 1024

func color(_ hex: UInt32) -> CGColor {
    NSColor(srgbRed: CGFloat((hex >> 16) & 0xff) / 255,
            green: CGFloat((hex >> 8) & 0xff) / 255,
            blue: CGFloat(hex & 0xff) / 255, alpha: 1).cgColor
}
func rrect(_ x: CGFloat, _ y: CGFloat, _ w: CGFloat, _ h: CGFloat, _ r: CGFloat) -> CGPath {
    CGPath(roundedRect: CGRect(x: x, y: y, width: w, height: h), cornerWidth: r, cornerHeight: r, transform: nil)
}

guard let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(S), pixelsHigh: Int(S),
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
    let gctx = NSGraphicsContext(bitmapImageRep: rep) else { exit(1) }

NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = gctx
let cg = gctx.cgContext

// Top-left origin (matches the SVG design coordinates).
cg.translateBy(x: 0, y: S)
cg.scaleBy(x: 1, y: -1)

// Rounded-square ground, inset with padding for macOS + a soft baked shadow.
let sq = rrect(100, 100, 824, 824, 185)
cg.saveGState()
cg.setShadow(offset: CGSize(width: 0, height: -20), blur: 36, color: color(0x000000).copy(alpha: 0.38))
cg.addPath(sq); cg.setFillColor(color(0x1A1A1E)); cg.fillPath()
cg.restoreGState()

// Subtle vertical gradient for depth.
cg.saveGState(); cg.addPath(sq); cg.clip()
let grad = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                      colors: [color(0x242429), color(0x141417)] as CFArray, locations: [0, 1])!
cg.drawLinearGradient(grad, start: CGPoint(x: 512, y: 100), end: CGPoint(x: 512, y: 924), options: [])
cg.restoreGState()

// Listening pill.
cg.addPath(rrect(176, 402, 672, 220, 110)); cg.setFillColor(color(0x2B2B31)); cg.fillPath()

// Amber record dot.
cg.addPath(CGPath(ellipseIn: CGRect(x: 276 - 38, y: 512 - 38, width: 76, height: 76), transform: nil))
cg.setFillColor(color(0xFFB020)); cg.fillPath()

// Off-white waveform.
let bars: [(CGFloat, CGFloat)] = [(380, 70), (440, 130), (500, 200), (560, 140), (620, 200), (680, 130), (740, 70)]
cg.setFillColor(color(0xECECF0))
for (bx, bh) in bars { cg.addPath(rrect(bx, 512 - bh / 2, 34, bh, 17)) }
cg.fillPath()

NSGraphicsContext.restoreGraphicsState()

let dir = (outPath as NSString).deletingLastPathComponent
if !dir.isEmpty { try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true) }
guard let png = rep.representation(using: .png, properties: [:]) else { exit(1) }
try! png.write(to: URL(fileURLWithPath: outPath))
print("wrote \(outPath)")
