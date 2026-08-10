import AppKit
import Foundation

// Draws the DMG window background (Graphite) with an arrow from app → Applications.
// Usage: swift Scripts/make-dmg-bg.swift [outPath]

let out = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "build/dmg-bg.png"
let W: CGFloat = 640, H: CGFloat = 400

func col(_ h: UInt32) -> NSColor {
    NSColor(srgbRed: CGFloat((h >> 16) & 0xff) / 255, green: CGFloat((h >> 8) & 0xff) / 255,
            blue: CGFloat(h & 0xff) / 255, alpha: 1)
}

guard let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(W), pixelsHigh: Int(H),
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
    let g = NSGraphicsContext(bitmapImageRep: rep) else { exit(1) }

NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = g
let cg = g.cgContext

// Light ground so Finder's black icon labels stay readable.
NSGradient(starting: col(0xF1F1F4), ending: col(0xFBFBFD))!
    .draw(in: NSRect(x: 0, y: 0, width: W, height: H), angle: 90)

// Amber arrow between the two icon columns (icons sit at y=205 from top → y-up = H-205).
let ay = H - 205
cg.setStrokeColor(col(0xF0A400).cgColor)
cg.setFillColor(col(0xF0A400).cgColor)
cg.setLineWidth(12); cg.setLineCap(.round)
cg.move(to: CGPoint(x: 262, y: ay)); cg.addLine(to: CGPoint(x: 372, y: ay)); cg.strokePath()
cg.move(to: CGPoint(x: 372, y: ay + 20)); cg.addLine(to: CGPoint(x: 400, y: ay))
cg.addLine(to: CGPoint(x: 372, y: ay - 20)); cg.closePath(); cg.fillPath()

func text(_ s: String, _ size: CGFloat, _ c: NSColor, _ weight: NSFont.Weight, centerY: CGFloat) {
    let para = NSMutableParagraphStyle(); para.alignment = .center
    let str = NSAttributedString(string: s, attributes: [
        .font: NSFont.systemFont(ofSize: size, weight: weight),
        .foregroundColor: c, .paragraphStyle: para,
    ])
    let sz = str.size()
    str.draw(in: NSRect(x: 0, y: centerY - sz.height / 2, width: W, height: sz.height))
}

text("Yap", 34, col(0x1B1B1F), .bold, centerY: H - 52)
text("Local-first voice dictation", 13, col(0x7C7C84), .regular, centerY: H - 86)

NSGraphicsContext.restoreGraphicsState()

let dir = (out as NSString).deletingLastPathComponent
if !dir.isEmpty { try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true) }
try! rep.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: out))
print("wrote \(out)")
