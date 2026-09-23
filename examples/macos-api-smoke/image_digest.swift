import AppKit
import Foundation

guard CommandLine.arguments.count == 2,
      let data = try? Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1])),
      let image = NSBitmapImageRep(data: data),
      let pixels = image.bitmapData else {
    fputs("usage: image_digest <image>\n", stderr)
    exit(2)
}

var digest: UInt64 = 1469598103934665603
for byte in UnsafeBufferPointer(start: pixels, count: image.bytesPerRow * image.pixelsHigh) {
    digest = (digest ^ UInt64(byte)) &* 1099511628211
}
print(String(format: "%016llx", digest))
