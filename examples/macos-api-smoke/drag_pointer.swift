import CoreGraphics
import Foundation

guard CommandLine.arguments.count == 5,
      let startX = Double(CommandLine.arguments[1]),
      let startY = Double(CommandLine.arguments[2]),
      let endX = Double(CommandLine.arguments[3]),
      let endY = Double(CommandLine.arguments[4]) else {
    fputs("usage: drag_pointer <start-x> <start-y> <end-x> <end-y>\n", stderr)
    exit(2)
}

let source = CGEventSource(stateID: .hidSystemState)
let start = CGPoint(x: startX, y: startY)
CGWarpMouseCursorPosition(start)
usleep(150_000)
CGEvent(mouseEventSource: source, mouseType: .leftMouseDown,
        mouseCursorPosition: start, mouseButton: .left)?.post(tap: .cghidEventTap)
usleep(500_000)
for step in 1...16 {
    let fraction = Double(step) / 16.0
    let point = CGPoint(x: startX + (endX - startX) * fraction,
                        y: startY + (endY - startY) * fraction)
    CGEvent(mouseEventSource: source, mouseType: .leftMouseDragged,
            mouseCursorPosition: point, mouseButton: .left)?.post(tap: .cghidEventTap)
    usleep(25_000)
}
let end = CGPoint(x: endX, y: endY)
CGEvent(mouseEventSource: source, mouseType: .leftMouseUp,
        mouseCursorPosition: end, mouseButton: .left)?.post(tap: .cghidEventTap)
