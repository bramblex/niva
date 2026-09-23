import AppKit
import Foundation

struct Snapshot: Codable, Equatable {
    var items: [[String: String]]
    var plainText: String?
    var changeCount: Int
}

func fail(_ message: String) -> Never {
    fputs("clipboard_state.swift: \(message)\n", stderr)
    exit(2)
}

func capture() -> Snapshot {
    let pasteboard = NSPasteboard.general
    let initialChangeCount = pasteboard.changeCount
    var rows: [[String: String]] = []
    for item in pasteboard.pasteboardItems ?? [] {
        var row: [String: String] = [:]
        for type in item.types {
            guard let data = item.data(forType: type) else {
                fail("cannot capture pasteboard representation \(type.rawValue)")
            }
            row[type.rawValue] = data.base64EncodedString()
        }
        rows.append(row)
    }
    let plainText = pasteboard.string(forType: .string)
    let finalChangeCount = pasteboard.changeCount
    guard finalChangeCount == initialChangeCount else {
        fail("pasteboard changed while its representations were being captured")
    }
    return Snapshot(items: rows, plainText: plainText, changeCount: finalChangeCount)
}

func isExactSentinel(_ pasteboard: NSPasteboard, _ sentinel: String) -> Bool {
    guard pasteboard.string(forType: .string) == sentinel,
          let items = pasteboard.pasteboardItems,
          items.count == 1,
          !items[0].types.isEmpty else {
        return false
    }
    for type in items[0].types {
        guard items[0].string(forType: type) == sentinel else { return false }
    }
    return true
}

func readSnapshot(_ path: String) -> Snapshot {
    do {
        return try JSONDecoder().decode(Snapshot.self, from: Data(contentsOf: URL(fileURLWithPath: path)))
    } catch {
        fail("cannot read snapshot: \(error)")
    }
}

func writeSnapshot(_ snapshot: Snapshot, to path: String) {
    do {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        let data = try encoder.encode(snapshot)
        try data.write(to: URL(fileURLWithPath: path), options: .atomic)
    } catch {
        fail("cannot write snapshot: \(error)")
    }
}

guard CommandLine.arguments.count >= 3 else {
    fail("usage: clipboard_state.swift snapshot PATH | matches-sentinel SENTINEL CHANGE_COUNT | restore-if-snapshot ORIGINAL EXPECTED | restore-if-sentinel ORIGINAL SENTINEL CHANGE_COUNT")
}

let operation = CommandLine.arguments[1]
let path = CommandLine.arguments[2]
switch operation {
case "snapshot":
    guard CommandLine.arguments.count == 3 else { fail("snapshot requires one output path") }
    writeSnapshot(capture(), to: path)
case "matches-sentinel":
    guard CommandLine.arguments.count == 4,
          let expectedChangeCount = Int(CommandLine.arguments[3]) else {
        fail("matches-sentinel requires SENTINEL and CHANGE_COUNT")
    }
    let pasteboard = NSPasteboard.general
    guard pasteboard.changeCount == expectedChangeCount,
          isExactSentinel(pasteboard, path),
          pasteboard.changeCount == expectedChangeCount else {
        exit(4)
    }
case "restore-if-snapshot":
    guard CommandLine.arguments.count == 4 else { fail("restore-if-snapshot requires ORIGINAL and EXPECTED paths") }
    let saved = readSnapshot(path)
    let expected = readSnapshot(CommandLine.arguments[3])
    let pasteboard = NSPasteboard.general
    guard capture() == expected else { exit(4) }
    restore(saved, to: pasteboard, ifChangeCount: expected.changeCount)
case "restore-if-sentinel":
    guard CommandLine.arguments.count == 5,
          let expectedChangeCount = Int(CommandLine.arguments[4]) else {
        fail("restore-if-sentinel requires ORIGINAL, SENTINEL, and CHANGE_COUNT")
    }
    let saved = readSnapshot(path)
    let sentinel = CommandLine.arguments[3]
    let pasteboard = NSPasteboard.general
    guard pasteboard.changeCount == expectedChangeCount,
          isExactSentinel(pasteboard, sentinel),
          pasteboard.changeCount == expectedChangeCount else {
        exit(4)
    }
    restore(saved, to: pasteboard, ifChangeCount: expectedChangeCount)
default:
    fail("unknown operation \(operation)")
}

func restore(_ saved: Snapshot, to pasteboard: NSPasteboard, ifChangeCount: Int) {
    guard pasteboard.changeCount == ifChangeCount else { exit(4) }
    var restored: [NSPasteboardItem] = []
    for row in saved.items {
        let item = NSPasteboardItem()
        for (rawType, encoded) in row {
            guard let data = Data(base64Encoded: encoded) else {
                fail("invalid base64 in saved representation \(rawType)")
            }
            item.setData(data, forType: NSPasteboard.PasteboardType(rawValue: rawType))
        }
        restored.append(item)
    }
    pasteboard.clearContents()
    if !restored.isEmpty && !pasteboard.writeObjects(restored) {
        fail("NSPasteboard rejected the saved item list")
    }
}
