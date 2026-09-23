import AppKit
import Foundation

guard CommandLine.arguments.count == 2,
      let data = try? Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1])),
      let image = NSBitmapImageRep(data: data) else {
    fputs("usage: dock_red_pixels <png>\n", stderr)
    exit(2)
}

var redPixels = 0
for y in 0..<image.pixelsHigh {
    for x in 0..<image.pixelsWide {
        guard let color = image.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB) else { continue }
        if color.redComponent > 0.60 && color.greenComponent < 0.42 && color.blueComponent < 0.42 {
            redPixels += 1
        }
    }
}
print(redPixels)
