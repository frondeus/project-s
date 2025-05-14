import XCTest
import SwiftTreeSitter
import TreeSitterS

final class TreeSitterSTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_s())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading S grammar")
    }
}
