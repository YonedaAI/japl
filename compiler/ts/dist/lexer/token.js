export var TokenKind;
(function (TokenKind) {
    // Literals
    TokenKind[TokenKind["Int"] = 0] = "Int";
    TokenKind[TokenKind["Float"] = 1] = "Float";
    TokenKind[TokenKind["Byte"] = 2] = "Byte";
    TokenKind[TokenKind["String"] = 3] = "String";
    TokenKind[TokenKind["True"] = 4] = "True";
    TokenKind[TokenKind["False"] = 5] = "False";
    // Identifiers
    TokenKind[TokenKind["Ident"] = 6] = "Ident";
    TokenKind[TokenKind["UpperIdent"] = 7] = "UpperIdent";
    // Keywords (38)
    TokenKind[TokenKind["Fn"] = 8] = "Fn";
    TokenKind[TokenKind["Let"] = 9] = "Let";
    TokenKind[TokenKind["Type"] = 10] = "Type";
    TokenKind[TokenKind["Match"] = 11] = "Match";
    TokenKind[TokenKind["If"] = 12] = "If";
    TokenKind[TokenKind["Else"] = 13] = "Else";
    TokenKind[TokenKind["Trait"] = 14] = "Trait";
    TokenKind[TokenKind["Impl"] = 15] = "Impl";
    TokenKind[TokenKind["Module"] = 16] = "Module";
    TokenKind[TokenKind["Import"] = 17] = "Import";
    TokenKind[TokenKind["Pub"] = 18] = "Pub";
    TokenKind[TokenKind["Opaque"] = 19] = "Opaque";
    TokenKind[TokenKind["Spawn"] = 20] = "Spawn";
    TokenKind[TokenKind["Send"] = 21] = "Send";
    TokenKind[TokenKind["Receive"] = 22] = "Receive";
    TokenKind[TokenKind["Supervisor"] = 23] = "Supervisor";
    TokenKind[TokenKind["Process"] = 24] = "Process";
    TokenKind[TokenKind["Test"] = 25] = "Test";
    TokenKind[TokenKind["Assert"] = 26] = "Assert";
    TokenKind[TokenKind["Foreign"] = 27] = "Foreign";
    TokenKind[TokenKind["Unsafe"] = 28] = "Unsafe";
    TokenKind[TokenKind["Use"] = 29] = "Use";
    TokenKind[TokenKind["Return"] = 30] = "Return";
    TokenKind[TokenKind["Done"] = 31] = "Done";
    TokenKind[TokenKind["Fail"] = 32] = "Fail";
    TokenKind[TokenKind["Panic"] = 33] = "Panic";
    TokenKind[TokenKind["Where"] = 34] = "Where";
    TokenKind[TokenKind["As"] = 35] = "As";
    TokenKind[TokenKind["In"] = 36] = "In";
    TokenKind[TokenKind["On"] = 37] = "On";
    TokenKind[TokenKind["With"] = 38] = "With";
    TokenKind[TokenKind["Strategy"] = 39] = "Strategy";
    TokenKind[TokenKind["Child"] = 40] = "Child";
    TokenKind[TokenKind["Resource"] = 41] = "Resource";
    TokenKind[TokenKind["Continue"] = 42] = "Continue";
    TokenKind[TokenKind["Property"] = 43] = "Property";
    TokenKind[TokenKind["Bench"] = 44] = "Bench";
    TokenKind[TokenKind["Tool"] = 45] = "Tool";
    // Operators
    TokenKind[TokenKind["Plus"] = 46] = "Plus";
    TokenKind[TokenKind["Minus"] = 47] = "Minus";
    TokenKind[TokenKind["Star"] = 48] = "Star";
    TokenKind[TokenKind["Slash"] = 49] = "Slash";
    TokenKind[TokenKind["Percent"] = 50] = "Percent";
    TokenKind[TokenKind["Eq"] = 51] = "Eq";
    TokenKind[TokenKind["NotEq"] = 52] = "NotEq";
    TokenKind[TokenKind["Lt"] = 53] = "Lt";
    TokenKind[TokenKind["Gt"] = 54] = "Gt";
    TokenKind[TokenKind["LtEq"] = 55] = "LtEq";
    TokenKind[TokenKind["GtEq"] = 56] = "GtEq";
    TokenKind[TokenKind["And"] = 57] = "And";
    TokenKind[TokenKind["Or"] = 58] = "Or";
    TokenKind[TokenKind["Not"] = 59] = "Not";
    TokenKind[TokenKind["Pipe"] = 60] = "Pipe";
    TokenKind[TokenKind["Compose"] = 61] = "Compose";
    TokenKind[TokenKind["Concat"] = 62] = "Concat";
    TokenKind[TokenKind["Arrow"] = 63] = "Arrow";
    TokenKind[TokenKind["FatArrow"] = 64] = "FatArrow";
    TokenKind[TokenKind["Question"] = 65] = "Question";
    TokenKind[TokenKind["Assign"] = 66] = "Assign";
    TokenKind[TokenKind["Bar"] = 67] = "Bar";
    TokenKind[TokenKind["Dot"] = 68] = "Dot";
    TokenKind[TokenKind["DotDot"] = 69] = "DotDot";
    TokenKind[TokenKind["Colon"] = 70] = "Colon";
    TokenKind[TokenKind["ColonColon"] = 71] = "ColonColon";
    TokenKind[TokenKind["Comma"] = 72] = "Comma";
    TokenKind[TokenKind["Semicolon"] = 73] = "Semicolon";
    TokenKind[TokenKind["Ampersand"] = 74] = "Ampersand";
    // Delimiters
    TokenKind[TokenKind["LParen"] = 75] = "LParen";
    TokenKind[TokenKind["RParen"] = 76] = "RParen";
    TokenKind[TokenKind["LBrace"] = 77] = "LBrace";
    TokenKind[TokenKind["RBrace"] = 78] = "RBrace";
    TokenKind[TokenKind["LBracket"] = 79] = "LBracket";
    TokenKind[TokenKind["RBracket"] = 80] = "RBracket";
    // Special
    TokenKind[TokenKind["Newline"] = 81] = "Newline";
    TokenKind[TokenKind["EOF"] = 82] = "EOF";
    TokenKind[TokenKind["Comment"] = 83] = "Comment";
})(TokenKind || (TokenKind = {}));
export const KEYWORDS = new Map([
    ['fn', TokenKind.Fn],
    ['let', TokenKind.Let],
    ['type', TokenKind.Type],
    ['match', TokenKind.Match],
    ['if', TokenKind.If],
    ['else', TokenKind.Else],
    ['trait', TokenKind.Trait],
    ['impl', TokenKind.Impl],
    ['module', TokenKind.Module],
    ['import', TokenKind.Import],
    ['pub', TokenKind.Pub],
    ['opaque', TokenKind.Opaque],
    ['spawn', TokenKind.Spawn],
    ['send', TokenKind.Send],
    ['receive', TokenKind.Receive],
    ['supervisor', TokenKind.Supervisor],
    ['process', TokenKind.Process],
    ['test', TokenKind.Test],
    ['assert', TokenKind.Assert],
    ['foreign', TokenKind.Foreign],
    ['unsafe', TokenKind.Unsafe],
    ['use', TokenKind.Use],
    ['return', TokenKind.Return],
    ['done', TokenKind.Done],
    ['fail', TokenKind.Fail],
    ['panic', TokenKind.Panic],
    ['where', TokenKind.Where],
    ['as', TokenKind.As],
    ['in', TokenKind.In],
    ['on', TokenKind.On],
    ['with', TokenKind.With],
    ['strategy', TokenKind.Strategy],
    ['child', TokenKind.Child],
    ['resource', TokenKind.Resource],
    ['continue', TokenKind.Continue],
    ['property', TokenKind.Property],
    ['bench', TokenKind.Bench],
    ['tool', TokenKind.Tool],
    ['true', TokenKind.True],
    ['false', TokenKind.False],
]);
const TOKEN_KIND_NAMES = {
    [TokenKind.Int]: 'Int',
    [TokenKind.Float]: 'Float',
    [TokenKind.Byte]: 'Byte',
    [TokenKind.String]: 'String',
    [TokenKind.True]: 'True',
    [TokenKind.False]: 'False',
    [TokenKind.Ident]: 'Ident',
    [TokenKind.UpperIdent]: 'UpperIdent',
    [TokenKind.Fn]: 'Fn',
    [TokenKind.Let]: 'Let',
    [TokenKind.Type]: 'Type',
    [TokenKind.Match]: 'Match',
    [TokenKind.If]: 'If',
    [TokenKind.Else]: 'Else',
    [TokenKind.Trait]: 'Trait',
    [TokenKind.Impl]: 'Impl',
    [TokenKind.Module]: 'Module',
    [TokenKind.Import]: 'Import',
    [TokenKind.Pub]: 'Pub',
    [TokenKind.Opaque]: 'Opaque',
    [TokenKind.Spawn]: 'Spawn',
    [TokenKind.Send]: 'Send',
    [TokenKind.Receive]: 'Receive',
    [TokenKind.Supervisor]: 'Supervisor',
    [TokenKind.Process]: 'Process',
    [TokenKind.Test]: 'Test',
    [TokenKind.Assert]: 'Assert',
    [TokenKind.Foreign]: 'Foreign',
    [TokenKind.Unsafe]: 'Unsafe',
    [TokenKind.Use]: 'Use',
    [TokenKind.Return]: 'Return',
    [TokenKind.Done]: 'Done',
    [TokenKind.Fail]: 'Fail',
    [TokenKind.Panic]: 'Panic',
    [TokenKind.Where]: 'Where',
    [TokenKind.As]: 'As',
    [TokenKind.In]: 'In',
    [TokenKind.On]: 'On',
    [TokenKind.With]: 'With',
    [TokenKind.Strategy]: 'Strategy',
    [TokenKind.Child]: 'Child',
    [TokenKind.Resource]: 'Resource',
    [TokenKind.Continue]: 'Continue',
    [TokenKind.Property]: 'Property',
    [TokenKind.Bench]: 'Bench',
    [TokenKind.Tool]: 'Tool',
    [TokenKind.Plus]: 'Plus',
    [TokenKind.Minus]: 'Minus',
    [TokenKind.Star]: 'Star',
    [TokenKind.Slash]: 'Slash',
    [TokenKind.Percent]: 'Percent',
    [TokenKind.Eq]: 'Eq',
    [TokenKind.NotEq]: 'NotEq',
    [TokenKind.Lt]: 'Lt',
    [TokenKind.Gt]: 'Gt',
    [TokenKind.LtEq]: 'LtEq',
    [TokenKind.GtEq]: 'GtEq',
    [TokenKind.And]: 'And',
    [TokenKind.Or]: 'Or',
    [TokenKind.Not]: 'Not',
    [TokenKind.Pipe]: 'Pipe',
    [TokenKind.Compose]: 'Compose',
    [TokenKind.Concat]: 'Concat',
    [TokenKind.Arrow]: 'Arrow',
    [TokenKind.FatArrow]: 'FatArrow',
    [TokenKind.Question]: 'Question',
    [TokenKind.Assign]: 'Assign',
    [TokenKind.Bar]: 'Bar',
    [TokenKind.Dot]: 'Dot',
    [TokenKind.DotDot]: 'DotDot',
    [TokenKind.Colon]: 'Colon',
    [TokenKind.ColonColon]: 'ColonColon',
    [TokenKind.Comma]: 'Comma',
    [TokenKind.Semicolon]: 'Semicolon',
    [TokenKind.Ampersand]: 'Ampersand',
    [TokenKind.LParen]: 'LParen',
    [TokenKind.RParen]: 'RParen',
    [TokenKind.LBrace]: 'LBrace',
    [TokenKind.RBrace]: 'RBrace',
    [TokenKind.LBracket]: 'LBracket',
    [TokenKind.RBracket]: 'RBracket',
    [TokenKind.Newline]: 'Newline',
    [TokenKind.EOF]: 'EOF',
    [TokenKind.Comment]: 'Comment',
};
export function tokenKindName(kind) {
    return TOKEN_KIND_NAMES[kind] ?? 'Unknown';
}
//# sourceMappingURL=token.js.map