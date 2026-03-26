export var TokenKind;
(function (TokenKind) {
    // Literals
    TokenKind[TokenKind["Int"] = 0] = "Int";
    TokenKind[TokenKind["Float"] = 1] = "Float";
    TokenKind[TokenKind["String"] = 2] = "String";
    TokenKind[TokenKind["True"] = 3] = "True";
    TokenKind[TokenKind["False"] = 4] = "False";
    // Identifiers
    TokenKind[TokenKind["Ident"] = 5] = "Ident";
    TokenKind[TokenKind["UpperIdent"] = 6] = "UpperIdent";
    // Keywords (37)
    TokenKind[TokenKind["Fn"] = 7] = "Fn";
    TokenKind[TokenKind["Let"] = 8] = "Let";
    TokenKind[TokenKind["Type"] = 9] = "Type";
    TokenKind[TokenKind["Match"] = 10] = "Match";
    TokenKind[TokenKind["If"] = 11] = "If";
    TokenKind[TokenKind["Else"] = 12] = "Else";
    TokenKind[TokenKind["Trait"] = 13] = "Trait";
    TokenKind[TokenKind["Impl"] = 14] = "Impl";
    TokenKind[TokenKind["Module"] = 15] = "Module";
    TokenKind[TokenKind["Import"] = 16] = "Import";
    TokenKind[TokenKind["Pub"] = 17] = "Pub";
    TokenKind[TokenKind["Opaque"] = 18] = "Opaque";
    TokenKind[TokenKind["Spawn"] = 19] = "Spawn";
    TokenKind[TokenKind["Send"] = 20] = "Send";
    TokenKind[TokenKind["Receive"] = 21] = "Receive";
    TokenKind[TokenKind["Supervisor"] = 22] = "Supervisor";
    TokenKind[TokenKind["Process"] = 23] = "Process";
    TokenKind[TokenKind["Test"] = 24] = "Test";
    TokenKind[TokenKind["Assert"] = 25] = "Assert";
    TokenKind[TokenKind["Foreign"] = 26] = "Foreign";
    TokenKind[TokenKind["Unsafe"] = 27] = "Unsafe";
    TokenKind[TokenKind["Use"] = 28] = "Use";
    TokenKind[TokenKind["Return"] = 29] = "Return";
    TokenKind[TokenKind["Done"] = 30] = "Done";
    TokenKind[TokenKind["Fail"] = 31] = "Fail";
    TokenKind[TokenKind["Panic"] = 32] = "Panic";
    TokenKind[TokenKind["Where"] = 33] = "Where";
    TokenKind[TokenKind["As"] = 34] = "As";
    TokenKind[TokenKind["In"] = 35] = "In";
    TokenKind[TokenKind["On"] = 36] = "On";
    TokenKind[TokenKind["With"] = 37] = "With";
    TokenKind[TokenKind["Strategy"] = 38] = "Strategy";
    TokenKind[TokenKind["Child"] = 39] = "Child";
    TokenKind[TokenKind["Resource"] = 40] = "Resource";
    TokenKind[TokenKind["Continue"] = 41] = "Continue";
    TokenKind[TokenKind["Property"] = 42] = "Property";
    TokenKind[TokenKind["Bench"] = 43] = "Bench";
    // Operators
    TokenKind[TokenKind["Plus"] = 44] = "Plus";
    TokenKind[TokenKind["Minus"] = 45] = "Minus";
    TokenKind[TokenKind["Star"] = 46] = "Star";
    TokenKind[TokenKind["Slash"] = 47] = "Slash";
    TokenKind[TokenKind["Percent"] = 48] = "Percent";
    TokenKind[TokenKind["Eq"] = 49] = "Eq";
    TokenKind[TokenKind["NotEq"] = 50] = "NotEq";
    TokenKind[TokenKind["Lt"] = 51] = "Lt";
    TokenKind[TokenKind["Gt"] = 52] = "Gt";
    TokenKind[TokenKind["LtEq"] = 53] = "LtEq";
    TokenKind[TokenKind["GtEq"] = 54] = "GtEq";
    TokenKind[TokenKind["And"] = 55] = "And";
    TokenKind[TokenKind["Or"] = 56] = "Or";
    TokenKind[TokenKind["Not"] = 57] = "Not";
    TokenKind[TokenKind["Pipe"] = 58] = "Pipe";
    TokenKind[TokenKind["Compose"] = 59] = "Compose";
    TokenKind[TokenKind["Concat"] = 60] = "Concat";
    TokenKind[TokenKind["Arrow"] = 61] = "Arrow";
    TokenKind[TokenKind["FatArrow"] = 62] = "FatArrow";
    TokenKind[TokenKind["Question"] = 63] = "Question";
    TokenKind[TokenKind["Assign"] = 64] = "Assign";
    TokenKind[TokenKind["Bar"] = 65] = "Bar";
    TokenKind[TokenKind["Dot"] = 66] = "Dot";
    TokenKind[TokenKind["DotDot"] = 67] = "DotDot";
    TokenKind[TokenKind["Colon"] = 68] = "Colon";
    TokenKind[TokenKind["ColonColon"] = 69] = "ColonColon";
    TokenKind[TokenKind["Comma"] = 70] = "Comma";
    TokenKind[TokenKind["Semicolon"] = 71] = "Semicolon";
    TokenKind[TokenKind["Ampersand"] = 72] = "Ampersand";
    // Delimiters
    TokenKind[TokenKind["LParen"] = 73] = "LParen";
    TokenKind[TokenKind["RParen"] = 74] = "RParen";
    TokenKind[TokenKind["LBrace"] = 75] = "LBrace";
    TokenKind[TokenKind["RBrace"] = 76] = "RBrace";
    TokenKind[TokenKind["LBracket"] = 77] = "LBracket";
    TokenKind[TokenKind["RBracket"] = 78] = "RBracket";
    // Special
    TokenKind[TokenKind["Newline"] = 79] = "Newline";
    TokenKind[TokenKind["EOF"] = 80] = "EOF";
    TokenKind[TokenKind["Comment"] = 81] = "Comment";
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
    ['true', TokenKind.True],
    ['false', TokenKind.False],
]);
const TOKEN_KIND_NAMES = {
    [TokenKind.Int]: 'Int',
    [TokenKind.Float]: 'Float',
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