import * as fs from 'node:fs';
function cons(x: any, xs: any[]): any[] { return [x, ...xs]; }
function append(xs: any[], ys: any[]): any[] { return [...xs, ...ys]; }
function char_at(s: string, i: number): string { return s[i] ?? ''; }
function string_length(s: string): number { return s.length; }
function substring(s: string, start: number, end: number): string { return s.slice(start, end); }
function read_file(filepath: string): string { return fs.readFileSync(filepath, 'utf-8'); }
function get_arg(n: number): string { return process.argv[n + 1] ?? ''; }
function println(s: string): void { console.log(s); }
function show(x: any): string { return String(x); }
type Token = { _tag: "TkInt"; _0: unknown } | { _tag: "TkFloat"; _0: unknown } | { _tag: "TkString"; _0: unknown } | { _tag: "TkIdent"; _0: unknown } | { _tag: "TkUpperIdent"; _0: unknown } | { _tag: "TkKeyword"; _0: unknown } | { _tag: "TkOperator"; _0: unknown } | { _tag: "TkLParen" } | { _tag: "TkRParen" } | { _tag: "TkLBrace" } | { _tag: "TkRBrace" } | { _tag: "TkLBracket" } | { _tag: "TkRBracket" } | { _tag: "TkComma" } | { _tag: "TkColon" } | { _tag: "TkSemicolon" } | { _tag: "TkArrow" } | { _tag: "TkFatArrow" } | { _tag: "TkPipe" } | { _tag: "TkBar" } | { _tag: "TkDot" } | { _tag: "TkDotDot" } | { _tag: "TkEof" };
const TkInt = (_0: unknown): Token => ({ _tag: "TkInt", _0 });
const TkFloat = (_0: unknown): Token => ({ _tag: "TkFloat", _0 });
const TkString = (_0: unknown): Token => ({ _tag: "TkString", _0 });
const TkIdent = (_0: unknown): Token => ({ _tag: "TkIdent", _0 });
const TkUpperIdent = (_0: unknown): Token => ({ _tag: "TkUpperIdent", _0 });
const TkKeyword = (_0: unknown): Token => ({ _tag: "TkKeyword", _0 });
const TkOperator = (_0: unknown): Token => ({ _tag: "TkOperator", _0 });
const TkLParen: Token = { _tag: "TkLParen" };
const TkRParen: Token = { _tag: "TkRParen" };
const TkLBrace: Token = { _tag: "TkLBrace" };
const TkRBrace: Token = { _tag: "TkRBrace" };
const TkLBracket: Token = { _tag: "TkLBracket" };
const TkRBracket: Token = { _tag: "TkRBracket" };
const TkComma: Token = { _tag: "TkComma" };
const TkColon: Token = { _tag: "TkColon" };
const TkSemicolon: Token = { _tag: "TkSemicolon" };
const TkArrow: Token = { _tag: "TkArrow" };
const TkFatArrow: Token = { _tag: "TkFatArrow" };
const TkPipe: Token = { _tag: "TkPipe" };
const TkBar: Token = { _tag: "TkBar" };
const TkDot: Token = { _tag: "TkDot" };
const TkDotDot: Token = { _tag: "TkDotDot" };
const TkEof: Token = { _tag: "TkEof" };

type Expr = { _tag: "EInt"; _0: unknown } | { _tag: "EFloat"; _0: unknown } | { _tag: "EString"; _0: unknown } | { _tag: "EBool"; _0: unknown } | { _tag: "EVar"; _0: unknown } | { _tag: "EApp"; _0: unknown; _1: unknown } | { _tag: "ELambda"; _0: unknown; _1: unknown } | { _tag: "ELet"; _0: unknown; _1: unknown; _2: unknown } | { _tag: "EMatch"; _0: unknown; _1: unknown } | { _tag: "EIf"; _0: unknown; _1: unknown; _2: unknown } | { _tag: "EBinOp"; _0: unknown; _1: unknown; _2: unknown } | { _tag: "EUnaryOp"; _0: unknown; _1: unknown } | { _tag: "ERecord"; _0: unknown } | { _tag: "EFieldAccess"; _0: unknown; _1: unknown } | { _tag: "EList"; _0: unknown } | { _tag: "EConstruct"; _0: unknown; _1: unknown } | { _tag: "EBlock"; _0: unknown } | { _tag: "EConcat"; _0: unknown; _1: unknown } | { _tag: "EPipe"; _0: unknown; _1: unknown };
const EInt = (_0: unknown): Expr => ({ _tag: "EInt", _0 });
const EFloat = (_0: unknown): Expr => ({ _tag: "EFloat", _0 });
const EString = (_0: unknown): Expr => ({ _tag: "EString", _0 });
const EBool = (_0: unknown): Expr => ({ _tag: "EBool", _0 });
const EVar = (_0: unknown): Expr => ({ _tag: "EVar", _0 });
const EApp = (_0: unknown, _1: unknown): Expr => ({ _tag: "EApp", _0, _1 });
const ELambda = (_0: unknown, _1: unknown): Expr => ({ _tag: "ELambda", _0, _1 });
const ELet = (_0: unknown, _1: unknown, _2: unknown): Expr => ({ _tag: "ELet", _0, _1, _2 });
const EMatch = (_0: unknown, _1: unknown): Expr => ({ _tag: "EMatch", _0, _1 });
const EIf = (_0: unknown, _1: unknown, _2: unknown): Expr => ({ _tag: "EIf", _0, _1, _2 });
const EBinOp = (_0: unknown, _1: unknown, _2: unknown): Expr => ({ _tag: "EBinOp", _0, _1, _2 });
const EUnaryOp = (_0: unknown, _1: unknown): Expr => ({ _tag: "EUnaryOp", _0, _1 });
const ERecord = (_0: unknown): Expr => ({ _tag: "ERecord", _0 });
const EFieldAccess = (_0: unknown, _1: unknown): Expr => ({ _tag: "EFieldAccess", _0, _1 });
const EList = (_0: unknown): Expr => ({ _tag: "EList", _0 });
const EConstruct = (_0: unknown, _1: unknown): Expr => ({ _tag: "EConstruct", _0, _1 });
const EBlock = (_0: unknown): Expr => ({ _tag: "EBlock", _0 });
const EConcat = (_0: unknown, _1: unknown): Expr => ({ _tag: "EConcat", _0, _1 });
const EPipe = (_0: unknown, _1: unknown): Expr => ({ _tag: "EPipe", _0, _1 });

type MatchArm = { _tag: "MkArm"; _0: unknown; _1: unknown };
const MkArm = (_0: unknown, _1: unknown): MatchArm => ({ _tag: "MkArm", _0, _1 });

type RecordField = { _tag: "MkField"; _0: unknown; _1: unknown };
const MkField = (_0: unknown, _1: unknown): RecordField => ({ _tag: "MkField", _0, _1 });

type Pattern = { _tag: "PVar"; _0: unknown } | { _tag: "PConstructor"; _0: unknown; _1: unknown } | { _tag: "PLiteral"; _0: unknown } | { _tag: "PWildcard" } | { _tag: "PList"; _0: unknown };
const PVar = (_0: unknown): Pattern => ({ _tag: "PVar", _0 });
const PConstructor = (_0: unknown, _1: unknown): Pattern => ({ _tag: "PConstructor", _0, _1 });
const PLiteral = (_0: unknown): Pattern => ({ _tag: "PLiteral", _0 });
const PWildcard: Pattern = { _tag: "PWildcard" };
const PList = (_0: unknown): Pattern => ({ _tag: "PList", _0 });

type Decl = { _tag: "DFn"; _0: unknown; _1: unknown; _2: unknown } | { _tag: "DType"; _0: unknown; _1: unknown } | { _tag: "DPubFn"; _0: unknown; _1: unknown; _2: unknown };
const DFn = (_0: unknown, _1: unknown, _2: unknown): Decl => ({ _tag: "DFn", _0, _1, _2 });
const DType = (_0: unknown, _1: unknown): Decl => ({ _tag: "DType", _0, _1 });
const DPubFn = (_0: unknown, _1: unknown, _2: unknown): Decl => ({ _tag: "DPubFn", _0, _1, _2 });

type Variant = { _tag: "MkVariant"; _0: unknown; _1: unknown };
const MkVariant = (_0: unknown, _1: unknown): Variant => ({ _tag: "MkVariant", _0, _1 });

type PR = { _tag: "MkPR"; _0: unknown; _1: unknown };
const MkPR = (_0: unknown, _1: unknown): PR => ({ _tag: "MkPR", _0, _1 });

type DeclR = { _tag: "MkDR"; _0: unknown; _1: unknown };
const MkDR = (_0: unknown, _1: unknown): DeclR => ({ _tag: "MkDR", _0, _1 });

type ListPR = { _tag: "MkLPR"; _0: unknown; _1: unknown };
const MkLPR = (_0: unknown, _1: unknown): ListPR => ({ _tag: "MkLPR", _0, _1 });

type StringListR = { _tag: "MkSLR"; _0: unknown; _1: unknown };
const MkSLR = (_0: unknown, _1: unknown): StringListR => ({ _tag: "MkSLR", _0, _1 });

type PatR = { _tag: "MkPatR"; _0: unknown; _1: unknown };
const MkPatR = (_0: unknown, _1: unknown): PatR => ({ _tag: "MkPatR", _0, _1 });

type PatListR = { _tag: "MkPLR"; _0: unknown; _1: unknown };
const MkPLR = (_0: unknown, _1: unknown): PatListR => ({ _tag: "MkPLR", _0, _1 });

type VarListR = { _tag: "MkVLR"; _0: unknown; _1: unknown };
const MkVLR = (_0: unknown, _1: unknown): VarListR => ({ _tag: "MkVLR", _0, _1 });

type IntR = { _tag: "MkIR"; _0: unknown; _1: unknown };
const MkIR = (_0: unknown, _1: unknown): IntR => ({ _tag: "MkIR", _0, _1 });

function is_digit(c) {
  if (c === "0") {
    return true;
  } else {
    if (c === "1") {
      return true;
    } else {
      if (c === "2") {
        return true;
      } else {
        if (c === "3") {
          return true;
        } else {
          if (c === "4") {
            return true;
          } else {
            if (c === "5") {
              return true;
            } else {
              if (c === "6") {
                return true;
              } else {
                if (c === "7") {
                  return true;
                } else {
                  if (c === "8") {
                    return true;
                  } else {
                    return c === "9" ? true : false;
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}

function is_lower(c) {
  if (c === "a") {
    return true;
  } else {
    if (c === "b") {
      return true;
    } else {
      if (c === "c") {
        return true;
      } else {
        if (c === "d") {
          return true;
        } else {
          if (c === "e") {
            return true;
          } else {
            if (c === "f") {
              return true;
            } else {
              if (c === "g") {
                return true;
              } else {
                if (c === "h") {
                  return true;
                } else {
                  if (c === "i") {
                    return true;
                  } else {
                    if (c === "j") {
                      return true;
                    } else {
                      if (c === "k") {
                        return true;
                      } else {
                        if (c === "l") {
                          return true;
                        } else {
                          if (c === "m") {
                            return true;
                          } else {
                            if (c === "n") {
                              return true;
                            } else {
                              if (c === "o") {
                                return true;
                              } else {
                                if (c === "p") {
                                  return true;
                                } else {
                                  if (c === "q") {
                                    return true;
                                  } else {
                                    if (c === "r") {
                                      return true;
                                    } else {
                                      if (c === "s") {
                                        return true;
                                      } else {
                                        if (c === "t") {
                                          return true;
                                        } else {
                                          if (c === "u") {
                                            return true;
                                          } else {
                                            if (c === "v") {
                                              return true;
                                            } else {
                                              if (c === "w") {
                                                return true;
                                              } else {
                                                if (c === "x") {
                                                  return true;
                                                } else {
                                                  if (c === "y") {
                                                    return true;
                                                  } else {
                                                    return c === "z" ? true : false;
                                                  }
                                                }
                                              }
                                            }
                                          }
                                        }
                                      }
                                    }
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}

function is_upper(c) {
  if (c === "A") {
    return true;
  } else {
    if (c === "B") {
      return true;
    } else {
      if (c === "C") {
        return true;
      } else {
        if (c === "D") {
          return true;
        } else {
          if (c === "E") {
            return true;
          } else {
            if (c === "F") {
              return true;
            } else {
              if (c === "G") {
                return true;
              } else {
                if (c === "H") {
                  return true;
                } else {
                  if (c === "I") {
                    return true;
                  } else {
                    if (c === "J") {
                      return true;
                    } else {
                      if (c === "K") {
                        return true;
                      } else {
                        if (c === "L") {
                          return true;
                        } else {
                          if (c === "M") {
                            return true;
                          } else {
                            if (c === "N") {
                              return true;
                            } else {
                              if (c === "O") {
                                return true;
                              } else {
                                if (c === "P") {
                                  return true;
                                } else {
                                  if (c === "Q") {
                                    return true;
                                  } else {
                                    if (c === "R") {
                                      return true;
                                    } else {
                                      if (c === "S") {
                                        return true;
                                      } else {
                                        if (c === "T") {
                                          return true;
                                        } else {
                                          if (c === "U") {
                                            return true;
                                          } else {
                                            if (c === "V") {
                                              return true;
                                            } else {
                                              if (c === "W") {
                                                return true;
                                              } else {
                                                if (c === "X") {
                                                  return true;
                                                } else {
                                                  if (c === "Y") {
                                                    return true;
                                                  } else {
                                                    return c === "Z" ? true : false;
                                                  }
                                                }
                                              }
                                            }
                                          }
                                        }
                                      }
                                    }
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}

function is_alpha(c) {
  if (is_lower(c)) {
    return true;
  } else {
    if (is_upper(c)) {
      return true;
    } else {
      return c === "_" ? true : false;
    }
  }
}

function is_alphanum(c) {
  return is_alpha(c) ? true : is_digit(c);
}

function is_whitespace(c) {
  if (c === " ") {
    return true;
  } else {
    if (c === "\t") {
      return true;
    } else {
      return c === "\r" ? true : false;
    }
  }
}

function is_keyword(s) {
  if (s === "fn") {
    return true;
  } else {
    if (s === "let") {
      return true;
    } else {
      if (s === "type") {
        return true;
      } else {
        if (s === "match") {
          return true;
        } else {
          if (s === "if") {
            return true;
          } else {
            if (s === "else") {
              return true;
            } else {
              if (s === "module") {
                return true;
              } else {
                if (s === "import") {
                  return true;
                } else {
                  return s === "pub" ? true : false;
                }
              }
            }
          }
        }
      }
    }
  }
}

function join(items, sep) {
  if (items.length === 0) {
    return "";
  } else if (items.length >= 1) {
    const x = items[0];
    const rest = items.slice(1);
    if (rest.length === 0) {
      return x;
    } else {
      return x + sep + join(rest, sep);
    }
  }
}

function repeat_string(s, n) {
  return n <= 0 ? "" : s + repeat_string(s, n - 1);
}

function tok_is_eof(tok) {
  if (tok._tag === "TkEof") {
    return true;
  } else {
    return false;
  }
}

function tok_keyword_val(tok) {
  if (tok._tag === "TkKeyword") {
    const s = tok._0;
    return s;
  } else {
    return "";
  }
}

function tok_ident_val(tok) {
  if (tok._tag === "TkIdent") {
    const s = tok._0;
    return s;
  } else {
    return "";
  }
}

function tok_upper_val(tok) {
  if (tok._tag === "TkUpperIdent") {
    const s = tok._0;
    return s;
  } else {
    return "";
  }
}

function tok_string_val(tok) {
  if (tok._tag === "TkString") {
    const s = tok._0;
    return s;
  } else {
    return "";
  }
}

function tok_int_val(tok) {
  if (tok._tag === "TkInt") {
    const s = tok._0;
    return s;
  } else {
    return "";
  }
}

function tok_float_val(tok) {
  if (tok._tag === "TkFloat") {
    const s = tok._0;
    return s;
  } else {
    return "";
  }
}

function tok_op_val(tok) {
  if (tok._tag === "TkOperator") {
    const s = tok._0;
    return s;
  } else {
    return "";
  }
}

function tok_is(tok, tag) {
  switch (tok._tag) {
    case "TkKeyword": {
      return tag === "keyword";
      break;
    }
    case "TkIdent": {
      return tag === "ident";
      break;
    }
    case "TkUpperIdent": {
      return tag === "upper";
      break;
    }
    case "TkOperator": {
      return tag === "operator";
      break;
    }
    case "TkInt": {
      return tag === "int";
      break;
    }
    case "TkFloat": {
      return tag === "float";
      break;
    }
    case "TkString": {
      return tag === "string";
      break;
    }
    case "TkLParen": {
      return tag === "lparen";
      break;
    }
    case "TkRParen": {
      return tag === "rparen";
      break;
    }
    case "TkLBrace": {
      return tag === "lbrace";
      break;
    }
    case "TkRBrace": {
      return tag === "rbrace";
      break;
    }
    case "TkLBracket": {
      return tag === "lbracket";
      break;
    }
    case "TkRBracket": {
      return tag === "rbracket";
      break;
    }
    case "TkComma": {
      return tag === "comma";
      break;
    }
    case "TkColon": {
      return tag === "colon";
      break;
    }
    case "TkSemicolon": {
      return tag === "semi";
      break;
    }
    case "TkArrow": {
      return tag === "arrow";
      break;
    }
    case "TkFatArrow": {
      return tag === "fatarrow";
      break;
    }
    case "TkPipe": {
      return tag === "pipe";
      break;
    }
    case "TkBar": {
      return tag === "bar";
      break;
    }
    case "TkDot": {
      return tag === "dot";
      break;
    }
    case "TkDotDot": {
      return tag === "dotdot";
      break;
    }
    case "TkEof": {
      return tag === "eof";
      break;
    }
  }
}

function hd(tokens) {
  if (tokens.length === 0) {
    return { _tag: "TkEof" };
  } else if (tokens.length >= 1) {
    const x = tokens[0];
    const rest = tokens.slice(1);
    return x;
  }
}

function tl(tokens) {
  if (tokens.length === 0) {
    return [];
  } else if (tokens.length >= 1) {
    const x = tokens[0];
    const rest = tokens.slice(1);
    return rest;
  }
}

function hd2(tokens) {
  return hd(tl(tokens));
}

function hd3(tokens) {
  return hd(tl(tl(tokens)));
}

function tl2(tokens) {
  return tl(tl(tokens));
}

function tl3(tokens) {
  return tl(tl(tl(tokens)));
}

function tokenize(source) {
  return lex(source, 0);
}

function lex(source, pos) {
  if (pos >= string_length(source)) {
    return [{ _tag: "TkEof" }];
  } else {
    const c = char_at(source, pos);
    if (c === "\n") {
      return lex(source, pos + 1);
    } else {
      if (is_whitespace(c)) {
        return lex(source, pos + 1);
      } else {
        if (c === "/") {
          return peek_char(source, pos + 1) === "/" ? lex_skip_comment(source, pos + 2) : cons({ _tag: "TkOperator", _0: "/" }, lex(source, pos + 1));
        } else {
          if (is_digit(c)) {
            return lex_number(source, pos);
          } else {
            if (is_alpha(c)) {
              return lex_ident(source, pos);
            } else {
              return c === "\"" ? lex_string(source, pos + 1, "") : lex_operator(source, pos);
            }
          }
        }
      }
    }
  }
}

function peek_char(source, pos) {
  return pos >= string_length(source) ? "" : char_at(source, pos);
}

function lex_skip_comment(source, pos) {
  if (pos >= string_length(source)) {
    return [{ _tag: "TkEof" }];
  } else {
    return char_at(source, pos) === "\n" ? lex(source, pos + 1) : lex_skip_comment(source, pos + 1);
  }
}

function lex_number(source, start) {
  const iend = collect_digits(source, start);
  if (peek_char(source, iend) === ".") {
    if (is_digit(peek_char(source, iend + 1))) {
      const fend = collect_digits(source, iend + 1);
      return cons({ _tag: "TkFloat", _0: substring(source, start, fend) }, lex(source, fend));
    } else {
      return cons({ _tag: "TkInt", _0: substring(source, start, iend) }, lex(source, iend));
    }
  } else {
    return cons({ _tag: "TkInt", _0: substring(source, start, iend) }, lex(source, iend));
  }
}

function collect_digits(source, pos) {
  if (pos >= string_length(source)) {
    return pos;
  } else {
    return is_digit(char_at(source, pos)) ? collect_digits(source, pos + 1) : pos;
  }
}

function lex_ident(source, start) {
  const iend = collect_alphanum(source, start);
  const word = substring(source, start, iend);
  if (word === "true") {
    return cons({ _tag: "TkKeyword", _0: "true" }, lex(source, iend));
  } else {
    if (word === "false") {
      return cons({ _tag: "TkKeyword", _0: "false" }, lex(source, iend));
    } else {
      if (is_keyword(word)) {
        return cons({ _tag: "TkKeyword", _0: word }, lex(source, iend));
      } else {
        return is_upper(char_at(word, 0)) ? cons({ _tag: "TkUpperIdent", _0: word }, lex(source, iend)) : cons({ _tag: "TkIdent", _0: word }, lex(source, iend));
      }
    }
  }
}

function collect_alphanum(source, pos) {
  if (pos >= string_length(source)) {
    return pos;
  } else {
    return is_alphanum(char_at(source, pos)) ? collect_alphanum(source, pos + 1) : pos;
  }
}

function lex_string(source, pos, acc) {
  if (pos >= string_length(source)) {
    return cons({ _tag: "TkString", _0: acc }, [{ _tag: "TkEof" }]);
  } else {
    const c = char_at(source, pos);
    if (c === "\"") {
      return cons({ _tag: "TkString", _0: acc }, lex(source, pos + 1));
    } else {
      if (c === "\\") {
        const escaped = char_at(source, pos + 1);
        return lex_string(source, pos + 2, acc + "\\" + escaped);
      } else {
        return lex_string(source, pos + 1, acc + c);
      }
    }
  }
}

function lex_operator(source, pos) {
  const c = char_at(source, pos);
  const next = peek_char(source, pos + 1);
  if (c === "(") {
    return cons({ _tag: "TkLParen" }, lex(source, pos + 1));
  } else {
    if (c === ")") {
      return cons({ _tag: "TkRParen" }, lex(source, pos + 1));
    } else {
      if (c === "{") {
        return cons({ _tag: "TkLBrace" }, lex(source, pos + 1));
      } else {
        if (c === "}") {
          return cons({ _tag: "TkRBrace" }, lex(source, pos + 1));
        } else {
          if (c === "[") {
            return cons({ _tag: "TkLBracket" }, lex(source, pos + 1));
          } else {
            if (c === "]") {
              return cons({ _tag: "TkRBracket" }, lex(source, pos + 1));
            } else {
              if (c === ",") {
                return cons({ _tag: "TkComma" }, lex(source, pos + 1));
              } else {
                if (c === ";") {
                  return cons({ _tag: "TkSemicolon" }, lex(source, pos + 1));
                } else {
                  if (c === ":") {
                    return cons({ _tag: "TkColon" }, lex(source, pos + 1));
                  } else {
                    if (c === ".") {
                      return next === "." ? cons({ _tag: "TkDotDot" }, lex(source, pos + 2)) : cons({ _tag: "TkDot" }, lex(source, pos + 1));
                    } else {
                      if (c === "-") {
                        return next === ">" ? cons({ _tag: "TkArrow" }, lex(source, pos + 2)) : cons({ _tag: "TkOperator", _0: "-" }, lex(source, pos + 1));
                      } else {
                        if (c === "=") {
                          if (next === ">") {
                            return cons({ _tag: "TkFatArrow" }, lex(source, pos + 2));
                          } else {
                            return next === "=" ? cons({ _tag: "TkOperator", _0: "==" }, lex(source, pos + 2)) : cons({ _tag: "TkOperator", _0: "=" }, lex(source, pos + 1));
                          }
                        } else {
                          if (c === "!") {
                            return next === "=" ? cons({ _tag: "TkOperator", _0: "!=" }, lex(source, pos + 2)) : cons({ _tag: "TkOperator", _0: "!" }, lex(source, pos + 1));
                          } else {
                            if (c === "<") {
                              if (next === ">") {
                                return cons({ _tag: "TkOperator", _0: "<>" }, lex(source, pos + 2));
                              } else {
                                return next === "=" ? cons({ _tag: "TkOperator", _0: "<=" }, lex(source, pos + 2)) : cons({ _tag: "TkOperator", _0: "<" }, lex(source, pos + 1));
                              }
                            } else {
                              if (c === ">") {
                                return next === "=" ? cons({ _tag: "TkOperator", _0: ">=" }, lex(source, pos + 2)) : cons({ _tag: "TkOperator", _0: ">" }, lex(source, pos + 1));
                              } else {
                                if (c === "|") {
                                  if (next === ">") {
                                    return cons({ _tag: "TkPipe" }, lex(source, pos + 2));
                                  } else {
                                    return next === "|" ? cons({ _tag: "TkOperator", _0: "||" }, lex(source, pos + 2)) : cons({ _tag: "TkBar" }, lex(source, pos + 1));
                                  }
                                } else {
                                  if (c === "&") {
                                    return next === "&" ? cons({ _tag: "TkOperator", _0: "&&" }, lex(source, pos + 2)) : lex(source, pos + 1);
                                  } else {
                                    if (c === "+") {
                                      return cons({ _tag: "TkOperator", _0: "+" }, lex(source, pos + 1));
                                    } else {
                                      if (c === "*") {
                                        return cons({ _tag: "TkOperator", _0: "*" }, lex(source, pos + 1));
                                      } else {
                                        return c === "%" ? cons({ _tag: "TkOperator", _0: "%" }, lex(source, pos + 1)) : lex(source, pos + 1);
                                      }
                                    }
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}

function parse(tokens) {
  return parse_decls(tokens);
}

function parse_decls(tokens) {
  if (tokens.length === 0) {
    return [];
  } else {
    const first = hd(tokens);
    if (tok_is_eof(first)) {
      return [];
    } else {
      if (first._tag === "TkKeyword") {
        const kw = first._0;
        if (kw === "pub") {
          const second = hd2(tokens);
          if (tok_keyword_val(second) === "fn") {
            const result = parse_fn_decl(tl2(tokens));
            switch (result._tag) {
              case "MkDR": {
                const rem = result._1;
                return cons({ _tag: "DPubFn", _0: name, _1: params, _2: body }, parse_decls(rem));
                break;
              }
              case "MkDR": {
                const d = result._0;
                const rem = result._1;
                return cons(d, parse_decls(rem));
                break;
              }
            }
          } else {
            return parse_decls(tl(tokens));
          }
        } else {
          if (kw === "fn") {
            const result = parse_fn_decl(tl(tokens));
            switch (result._tag) {
              case "MkDR": {
                const d = result._0;
                const rem = result._1;
                return cons(d, parse_decls(rem));
                break;
              }
            }
          } else {
            if (kw === "type") {
              const result = parse_type_decl(tl(tokens));
              switch (result._tag) {
                case "MkDR": {
                  const d = result._0;
                  const rem = result._1;
                  return cons(d, parse_decls(rem));
                  break;
                }
              }
            } else {
              return parse_decls(tl(tokens));
            }
          }
        }
      } else {
        return parse_decls(tl(tokens));
      }
    }
  }
}

function parse_fn_decl(tokens) {
  const name = tok_ident_val(hd(tokens));
  if (tok_is(hd2(tokens), "lparen")) {
    const pr = parse_param_names(tl2(tokens));
    switch (pr._tag) {
      case "MkSLR": {
        const params = pr._0;
        const rest2 = pr._1;
        const rest3 = skip_return_type(rest2);
        if (tok_is(hd(rest3), "lbrace")) {
          const br = parse_block_body(tl(rest3));
          switch (br._tag) {
            case "MkPR": {
              const body = br._0;
              const rest4 = br._1;
              return { _tag: "MkDR", _0: { _tag: "DFn", _0: name, _1: params, _2: body }, _1: rest4 };
              break;
            }
          }
        } else {
          return { _tag: "MkDR", _0: { _tag: "DFn", _0: name, _1: params, _2: { _tag: "EVar", _0: "unit" } }, _1: rest3 };
        }
        break;
      }
    }
  } else {
    return { _tag: "MkDR", _0: { _tag: "DFn", _0: "unknown", _1: [], _2: { _tag: "EVar", _0: "unit" } }, _1: tokens };
  }
}

function skip_return_type(tokens) {
  return tok_is(hd(tokens), "arrow") ? skip_type_expr(tl(tokens)) : tokens;
}

function skip_type_expr(tokens) {
  const first = hd(tokens);
  if (first._tag === "TkUpperIdent") {
    return tok_is(hd2(tokens), "lparen") ? skip_parens(tl2(tokens), 1) : tl(tokens);
  } else if (first._tag === "TkIdent") {
    return tok_is(hd2(tokens), "lparen") ? skip_parens(tl2(tokens), 1) : tl(tokens);
  } else if (first._tag === "TkLBrace") {
    return skip_braces(tl(tokens), 1);
  } else if (first._tag === "TkKeyword") {
    const kw = first._0;
    if (kw === "fn") {
      if (tok_is(hd2(tokens), "lparen")) {
        const after = skip_parens(tl2(tokens), 1);
        return tok_is(hd(after), "arrow") ? skip_type_expr(tl(after)) : after;
      } else {
        return tl(tokens);
      }
    } else {
      return tokens;
    }
  } else {
    return tokens;
  }
}

function skip_parens(tokens, depth) {
  if (depth === 0) {
    return tokens;
  } else {
    if (tokens.length === 0) {
      return [];
    } else {
      const first = hd(tokens);
      if (first._tag === "TkLParen") {
        return skip_parens(tl(tokens), depth + 1);
      } else if (first._tag === "TkRParen") {
        return skip_parens(tl(tokens), depth - 1);
      } else {
        return skip_parens(tl(tokens), depth);
      }
    }
  }
}

function skip_braces(tokens, depth) {
  if (depth === 0) {
    return tokens;
  } else {
    if (tokens.length === 0) {
      return [];
    } else {
      const first = hd(tokens);
      if (first._tag === "TkLBrace") {
        return skip_braces(tl(tokens), depth + 1);
      } else if (first._tag === "TkRBrace") {
        return skip_braces(tl(tokens), depth - 1);
      } else {
        return skip_braces(tl(tokens), depth);
      }
    }
  }
}

function parse_param_names(tokens) {
  const first = hd(tokens);
  if (first._tag === "TkRParen") {
    return { _tag: "MkSLR", _0: [], _1: tl(tokens) };
  } else if (first._tag === "TkIdent") {
    const name = first._0;
    const second = hd2(tokens);
    if (second._tag === "TkColon") {
      const rest2 = skip_type_annotation(tl2(tokens));
      if (tok_is(hd(rest2), "comma")) {
        const next = parse_param_names(tl(rest2));
        switch (next._tag) {
          case "MkSLR": {
            const params = next._0;
            const rem = next._1;
            return { _tag: "MkSLR", _0: cons(name, params), _1: rem };
            break;
          }
        }
      } else {
        return tok_is(hd(rest2), "rparen") ? { _tag: "MkSLR", _0: [name], _1: tl(rest2) } : { _tag: "MkSLR", _0: [name], _1: rest2 };
      }
    } else if (second._tag === "TkComma") {
      const next = parse_param_names(tl2(tokens));
      switch (next._tag) {
        case "MkSLR": {
          const params = next._0;
          const rem = next._1;
          return { _tag: "MkSLR", _0: cons(name, params), _1: rem };
          break;
        }
      }
    } else if (second._tag === "TkRParen") {
      return { _tag: "MkSLR", _0: [name], _1: tl2(tokens) };
    } else {
      return { _tag: "MkSLR", _0: [], _1: tokens };
    }
  } else {
    return { _tag: "MkSLR", _0: [], _1: tokens };
  }
}

function skip_type_annotation(tokens) {
  const first = hd(tokens);
  if (first._tag === "TkUpperIdent") {
    return tok_is(hd2(tokens), "lparen") ? skip_parens(tl2(tokens), 1) : tl(tokens);
  } else if (first._tag === "TkIdent") {
    return tok_is(hd2(tokens), "lparen") ? skip_parens(tl2(tokens), 1) : tl(tokens);
  } else if (first._tag === "TkKeyword") {
    const kw = first._0;
    if (kw === "fn") {
      if (tok_is(hd2(tokens), "lparen")) {
        const skipped = skip_parens(tl2(tokens), 1);
        return tok_is(hd(skipped), "arrow") ? skip_type_annotation(tl(skipped)) : skipped;
      } else {
        return tl(tokens);
      }
    } else {
      return tokens;
    }
  } else {
    return tokens;
  }
}

function parse_type_decl(tokens) {
  const first = hd(tokens);
  const name = tok_upper_val(first);
  const second = hd2(tokens);
  if (second._tag === "TkLParen") {
    const skipped = skip_parens(tl2(tokens), 1);
    if (tok_is(hd(skipped), "operator")) {
      if (tok_op_val(hd(skipped)) === "=") {
        const vr = parse_variants(tl(skipped));
        switch (vr._tag) {
          case "MkVLR": {
            const variants = vr._0;
            const rem = vr._1;
            return { _tag: "MkDR", _0: { _tag: "DType", _0: name, _1: variants }, _1: rem };
            break;
          }
        }
      } else {
        return { _tag: "MkDR", _0: { _tag: "DType", _0: name, _1: [] }, _1: skipped };
      }
    } else {
      return { _tag: "MkDR", _0: { _tag: "DType", _0: name, _1: [] }, _1: skipped };
    }
  } else if (second._tag === "TkOperator") {
    const eq = second._0;
    if (eq === "=") {
      const vr = parse_variants(tl2(tokens));
      switch (vr._tag) {
        case "MkVLR": {
          const variants = vr._0;
          const rem = vr._1;
          return { _tag: "MkDR", _0: { _tag: "DType", _0: name, _1: variants }, _1: rem };
          break;
        }
      }
    } else {
      return { _tag: "MkDR", _0: { _tag: "DType", _0: name, _1: [] }, _1: tl2(tokens) };
    }
  } else {
    return { _tag: "MkDR", _0: { _tag: "DType", _0: "Unknown", _1: [] }, _1: tokens };
  }
}

function parse_variants(tokens) {
  if (tok_is(hd(tokens), "bar")) {
    const name_tok = hd2(tokens);
    if (name_tok._tag === "TkUpperIdent") {
      const name = name_tok._0;
      if (tok_is(hd3(tokens), "lparen")) {
        const ar = count_variant_fields(tl3(tokens), 0);
        switch (ar._tag) {
          case "MkIR": {
            const arity = ar._0;
            const rest2 = ar._1;
            const next = parse_variants(rest2);
            switch (next._tag) {
              case "MkVLR": {
                const variants = next._0;
                const rem = next._1;
                return { _tag: "MkVLR", _0: cons({ _tag: "MkVariant", _0: name, _1: arity }, variants), _1: rem };
                break;
              }
            }
            break;
          }
        }
      } else {
        const next = parse_variants(tl2(tokens));
        switch (next._tag) {
          case "MkVLR": {
            const variants = next._0;
            const rem = next._1;
            return { _tag: "MkVLR", _0: cons({ _tag: "MkVariant", _0: name, _1: 0 }, variants), _1: rem };
            break;
          }
        }
      }
    } else {
      return { _tag: "MkVLR", _0: [], _1: tokens };
    }
  } else {
    return { _tag: "MkVLR", _0: [], _1: tokens };
  }
}

function count_variant_fields(tokens, count) {
  const first = hd(tokens);
  if (first._tag === "TkRParen") {
    return count === 0 ? { _tag: "MkIR", _0: 1, _1: tl(tokens) } : { _tag: "MkIR", _0: count, _1: tl(tokens) };
  } else if (first._tag === "TkComma") {
    return count_variant_fields(tl(tokens), count + 1);
  } else if (first._tag === "TkEof") {
    return { _tag: "MkIR", _0: count, _1: tokens };
  } else {
    return count === 0 ? count_variant_fields(tl(tokens), 1) : count_variant_fields(tl(tokens), count);
  }
}

function parse_block_body(tokens) {
  const result = parse_block_exprs(tokens);
  switch (result._tag) {
    case "MkLPR": {
      const exprs = result._0;
      const rest = result._1;
      if (exprs.length >= 1) {
        const single = exprs[0];
        const more = exprs.slice(1);
        if (more.length === 0) {
          return { _tag: "MkPR", _0: single, _1: rest };
        } else {
          return { _tag: "MkPR", _0: { _tag: "EBlock", _0: exprs }, _1: rest };
        }
      } else {
        return { _tag: "MkPR", _0: { _tag: "EBlock", _0: exprs }, _1: rest };
      }
      break;
    }
  }
}

function parse_block_exprs(tokens) {
  if (tokens.length === 0) {
    return { _tag: "MkLPR", _0: [], _1: [] };
  } else {
    if (tok_is(hd(tokens), "rbrace")) {
      return { _tag: "MkLPR", _0: [], _1: tl(tokens) };
    } else {
      const er = parse_expr(tokens);
      switch (er._tag) {
        case "MkPR": {
          const expr = er._0;
          const rest = er._1;
          const rest2 = skip_sep(rest);
          const next = parse_block_exprs(rest2);
          switch (next._tag) {
            case "MkLPR": {
              const exprs = next._0;
              const rem = next._1;
              return { _tag: "MkLPR", _0: cons(expr, exprs), _1: rem };
              break;
            }
          }
          break;
        }
      }
    }
  }
}

function skip_sep(tokens) {
  return tok_is(hd(tokens), "semi") ? tl(tokens) : tokens;
}

function parse_expr(tokens) {
  const lr = parse_primary(tokens);
  switch (lr._tag) {
    case "MkPR": {
      const lhs = lr._0;
      const rest = lr._1;
      return parse_expr_cont(lhs, rest);
      break;
    }
  }
}

function parse_expr_cont(lhs, tokens) {
  const first = hd(tokens);
  if (first._tag === "TkOperator") {
    const op = first._0;
    if (op === "<>") {
      const rr = parse_primary(tl(tokens));
      switch (rr._tag) {
        case "MkPR": {
          const rhs = rr._0;
          const rest = rr._1;
          return parse_expr_cont({ _tag: "EConcat", _0: lhs, _1: rhs }, rest);
          break;
        }
      }
    } else {
      if (op === "+" || op === "-" || op === "*" || op === "/" || op === "%" || op === "==" || op === "!=" || op === "<" || op === ">" || op === "<=" || op === ">=" || op === "&&" || op === "||") {
        const rr = parse_primary(tl(tokens));
        switch (rr._tag) {
          case "MkPR": {
            const rhs = rr._0;
            const rest = rr._1;
            return parse_expr_cont({ _tag: "EBinOp", _0: op, _1: lhs, _2: rhs }, rest);
            break;
          }
        }
      } else {
        return { _tag: "MkPR", _0: lhs, _1: tokens };
      }
    }
  } else if (first._tag === "TkPipe") {
    const rr = parse_primary(tl(tokens));
    switch (rr._tag) {
      case "MkPR": {
        const rhs = rr._0;
        const rest = rr._1;
        return parse_expr_cont({ _tag: "EPipe", _0: lhs, _1: rhs }, rest);
        break;
      }
    }
  } else if (first._tag === "TkDot") {
    const field_tok = hd2(tokens);
    const field = tok_ident_val(field_tok);
    return parse_expr_cont({ _tag: "EFieldAccess", _0: lhs, _1: field }, tl2(tokens));
  } else if (first._tag === "TkLParen") {
    const ar = parse_args(tl(tokens));
    switch (ar._tag) {
      case "MkLPR": {
        const args = ar._0;
        const rest = ar._1;
        return parse_expr_cont({ _tag: "EApp", _0: lhs, _1: args }, rest);
        break;
      }
    }
  } else {
    return { _tag: "MkPR", _0: lhs, _1: tokens };
  }
}

function parse_primary(tokens) {
  const first = hd(tokens);
  if (first._tag === "TkInt") {
    const n = first._0;
    return { _tag: "MkPR", _0: { _tag: "EInt", _0: n }, _1: tl(tokens) };
  } else if (first._tag === "TkFloat") {
    const n = first._0;
    return { _tag: "MkPR", _0: { _tag: "EFloat", _0: n }, _1: tl(tokens) };
  } else if (first._tag === "TkString") {
    const s = first._0;
    return { _tag: "MkPR", _0: { _tag: "EString", _0: s }, _1: tl(tokens) };
  } else if (first._tag === "TkKeyword") {
    const kw = first._0;
    if (kw === "true") {
      return { _tag: "MkPR", _0: { _tag: "EBool", _0: true }, _1: tl(tokens) };
    } else {
      if (kw === "false") {
        return { _tag: "MkPR", _0: { _tag: "EBool", _0: false }, _1: tl(tokens) };
      } else {
        if (kw === "let") {
          return parse_let(tl(tokens));
        } else {
          if (kw === "match") {
            return parse_match(tl(tokens));
          } else {
            if (kw === "if") {
              return parse_if(tl(tokens));
            } else {
              if (kw === "fn") {
                return tok_is(hd2(tokens), "lparen") ? parse_lambda(tl2(tokens)) : { _tag: "MkPR", _0: { _tag: "EVar", _0: "fn" }, _1: tl(tokens) };
              } else {
                return { _tag: "MkPR", _0: { _tag: "EVar", _0: kw }, _1: tl(tokens) };
              }
            }
          }
        }
      }
    }
  } else if (first._tag === "TkUpperIdent") {
    const name = first._0;
    if (tok_is(hd2(tokens), "lparen")) {
      const ar = parse_args(tl2(tokens));
      switch (ar._tag) {
        case "MkLPR": {
          const args = ar._0;
          const rest = ar._1;
          return { _tag: "MkPR", _0: { _tag: "EConstruct", _0: name, _1: args }, _1: rest };
          break;
        }
      }
    } else {
      return { _tag: "MkPR", _0: { _tag: "EConstruct", _0: name, _1: [] }, _1: tl(tokens) };
    }
  } else if (first._tag === "TkIdent") {
    const name = first._0;
    if (tok_is(hd2(tokens), "lparen")) {
      const ar = parse_args(tl2(tokens));
      switch (ar._tag) {
        case "MkLPR": {
          const args = ar._0;
          const rest = ar._1;
          return parse_expr_cont({ _tag: "EApp", _0: { _tag: "EVar", _0: name }, _1: args }, rest);
          break;
        }
      }
    } else {
      return { _tag: "MkPR", _0: { _tag: "EVar", _0: name }, _1: tl(tokens) };
    }
  } else if (first._tag === "TkLParen") {
    const er = parse_expr(tl(tokens));
    switch (er._tag) {
      case "MkPR": {
        const expr = er._0;
        const rest = er._1;
        return tok_is(hd(rest), "rparen") ? { _tag: "MkPR", _0: expr, _1: tl(rest) } : { _tag: "MkPR", _0: expr, _1: rest };
        break;
      }
    }
  } else if (first._tag === "TkLBrace") {
    return parse_record_or_block(tl(tokens));
  } else if (first._tag === "TkLBracket") {
    return parse_list_expr(tl(tokens));
  } else if (first._tag === "TkOperator") {
    const op = first._0;
    if (op === "-" || op === "!") {
      const er = parse_primary(tl(tokens));
      switch (er._tag) {
        case "MkPR": {
          const expr = er._0;
          const rest = er._1;
          return { _tag: "MkPR", _0: { _tag: "EUnaryOp", _0: op, _1: expr }, _1: rest };
          break;
        }
      }
    } else {
      return { _tag: "MkPR", _0: { _tag: "EVar", _0: "_error" }, _1: tokens };
    }
  } else {
    return { _tag: "MkPR", _0: { _tag: "EVar", _0: "_error" }, _1: tokens };
  }
}

function parse_args(tokens) {
  if (tok_is(hd(tokens), "rparen")) {
    return { _tag: "MkLPR", _0: [], _1: tl(tokens) };
  } else {
    const er = parse_expr(tokens);
    switch (er._tag) {
      case "MkPR": {
        const expr = er._0;
        const rest = er._1;
        if (tok_is(hd(rest), "comma")) {
          const next = parse_args(tl(rest));
          switch (next._tag) {
            case "MkLPR": {
              const args = next._0;
              const rem = next._1;
              return { _tag: "MkLPR", _0: cons(expr, args), _1: rem };
              break;
            }
          }
        } else {
          return tok_is(hd(rest), "rparen") ? { _tag: "MkLPR", _0: [expr], _1: tl(rest) } : { _tag: "MkLPR", _0: [expr], _1: rest };
        }
        break;
      }
    }
  }
}

function parse_let(tokens) {
  const name = tok_ident_val(hd(tokens));
  const second = hd2(tokens);
  if (second._tag === "TkOperator") {
    const eq = second._0;
    if (eq === "=") {
      const vr = parse_expr(tl2(tokens));
      switch (vr._tag) {
        case "MkPR": {
          const value = vr._0;
          const rest = vr._1;
          const rest2 = skip_sep(rest);
          const br = parse_expr(rest2);
          switch (br._tag) {
            case "MkPR": {
              const body = br._0;
              const rem = br._1;
              return { _tag: "MkPR", _0: { _tag: "ELet", _0: name, _1: value, _2: body }, _1: rem };
              break;
            }
          }
          break;
        }
      }
    } else {
      return { _tag: "MkPR", _0: { _tag: "EVar", _0: "_error" }, _1: tl(tokens) };
    }
  } else if (second._tag === "TkColon") {
    const rest2 = skip_type_annotation(tl2(tokens));
    if (tok_is(hd(rest2), "operator")) {
      if (tok_op_val(hd(rest2)) === "=") {
        const vr = parse_expr(tl(rest2));
        switch (vr._tag) {
          case "MkPR": {
            const value = vr._0;
            const rest3 = vr._1;
            const rest4 = skip_sep(rest3);
            const br = parse_expr(rest4);
            switch (br._tag) {
              case "MkPR": {
                const body = br._0;
                const rem = br._1;
                return { _tag: "MkPR", _0: { _tag: "ELet", _0: name, _1: value, _2: body }, _1: rem };
                break;
              }
            }
            break;
          }
        }
      } else {
        return { _tag: "MkPR", _0: { _tag: "EVar", _0: "_error" }, _1: rest2 };
      }
    } else {
      return { _tag: "MkPR", _0: { _tag: "EVar", _0: "_error" }, _1: rest2 };
    }
  } else {
    return { _tag: "MkPR", _0: { _tag: "EVar", _0: "_error" }, _1: tokens };
  }
}

function parse_match(tokens) {
  const sr = parse_primary(tokens);
  switch (sr._tag) {
    case "MkPR": {
      const scrutinee = sr._0;
      const rest = sr._1;
      if (tok_is(hd(rest), "lbrace")) {
        const ar = parse_match_arms(tl(rest));
        switch (ar._tag) {
          case "MkLPR": {
            const arms = ar._0;
            const rem = ar._1;
            return { _tag: "MkPR", _0: { _tag: "EMatch", _0: scrutinee, _1: arms }, _1: rem };
            break;
          }
        }
      } else {
        return { _tag: "MkPR", _0: { _tag: "EMatch", _0: scrutinee, _1: [] }, _1: rest };
      }
      break;
    }
  }
}

function parse_match_arms(tokens) {
  if (tok_is(hd(tokens), "rbrace")) {
    return { _tag: "MkLPR", _0: [], _1: tl(tokens) };
  } else {
    const pr = parse_pattern(tokens);
    switch (pr._tag) {
      case "MkPatR": {
        const pattern = pr._0;
        const rest = pr._1;
        if (tok_is(hd(rest), "fatarrow")) {
          const br = parse_expr(tl(rest));
          switch (br._tag) {
            case "MkPR": {
              const body = br._0;
              const rest2 = br._1;
              const rest3 = skip_comma(rest2);
              const next = parse_match_arms(rest3);
              switch (next._tag) {
                case "MkLPR": {
                  const arms = next._0;
                  const rem = next._1;
                  return { _tag: "MkLPR", _0: cons({ _tag: "MkArm", _0: pattern, _1: body }, arms), _1: rem };
                  break;
                }
              }
              break;
            }
          }
        } else {
          return { _tag: "MkLPR", _0: [], _1: rest };
        }
        break;
      }
    }
  }
}

function skip_comma(tokens) {
  return tok_is(hd(tokens), "comma") ? tl(tokens) : tokens;
}

function parse_pattern(tokens) {
  const first = hd(tokens);
  if (first._tag === "TkUpperIdent") {
    const name = first._0;
    if (tok_is(hd2(tokens), "lparen")) {
      const ar = parse_pattern_args(tl2(tokens));
      switch (ar._tag) {
        case "MkPLR": {
          const args = ar._0;
          const rest = ar._1;
          return { _tag: "MkPatR", _0: { _tag: "PConstructor", _0: name, _1: args }, _1: rest };
          break;
        }
      }
    } else {
      return { _tag: "MkPatR", _0: { _tag: "PConstructor", _0: name, _1: [] }, _1: tl(tokens) };
    }
  } else if (first._tag === "TkIdent") {
    const name = first._0;
    return name === "_" ? { _tag: "MkPatR", _0: { _tag: "PWildcard" }, _1: tl(tokens) } : { _tag: "MkPatR", _0: { _tag: "PVar", _0: name }, _1: tl(tokens) };
  } else if (first._tag === "TkInt") {
    const n = first._0;
    return { _tag: "MkPatR", _0: { _tag: "PLiteral", _0: { _tag: "EInt", _0: n } }, _1: tl(tokens) };
  } else if (first._tag === "TkString") {
    const s = first._0;
    return { _tag: "MkPatR", _0: { _tag: "PLiteral", _0: { _tag: "EString", _0: s } }, _1: tl(tokens) };
  } else if (first._tag === "TkLBracket") {
    return parse_list_pattern(tl(tokens));
  } else {
    return { _tag: "MkPatR", _0: { _tag: "PWildcard" }, _1: tokens };
  }
}

function parse_pattern_args(tokens) {
  if (tok_is(hd(tokens), "rparen")) {
    return { _tag: "MkPLR", _0: [], _1: tl(tokens) };
  } else {
    const pr = parse_pattern(tokens);
    switch (pr._tag) {
      case "MkPatR": {
        const pat = pr._0;
        const rest = pr._1;
        if (tok_is(hd(rest), "comma")) {
          const next = parse_pattern_args(tl(rest));
          switch (next._tag) {
            case "MkPLR": {
              const pats = next._0;
              const rem = next._1;
              return { _tag: "MkPLR", _0: cons(pat, pats), _1: rem };
              break;
            }
          }
        } else {
          return tok_is(hd(rest), "rparen") ? { _tag: "MkPLR", _0: [pat], _1: tl(rest) } : { _tag: "MkPLR", _0: [pat], _1: rest };
        }
        break;
      }
    }
  }
}

function parse_list_pattern(tokens) {
  if (tok_is(hd(tokens), "rbracket")) {
    return { _tag: "MkPatR", _0: { _tag: "PList", _0: [] }, _1: tl(tokens) };
  } else {
    const pr = parse_pattern(tokens);
    switch (pr._tag) {
      case "MkPatR": {
        const pat = pr._0;
        const rest = pr._1;
        if (tok_is(hd(rest), "comma")) {
          if (tok_is(hd2(rest), "dotdot")) {
            return { _tag: "MkPatR", _0: { _tag: "PList", _0: [pat] }, _1: tl(tl(tl(tl(rest)))) };
          } else {
            const next = parse_list_pattern_cont(tl(rest), [pat]);
            return next;
          }
        } else {
          return tok_is(hd(rest), "rbracket") ? { _tag: "MkPatR", _0: { _tag: "PList", _0: [pat] }, _1: tl(rest) } : { _tag: "MkPatR", _0: { _tag: "PList", _0: [pat] }, _1: rest };
        }
        break;
      }
    }
  }
}

function parse_list_pattern_cont(tokens, acc) {
  if (tok_is(hd(tokens), "rbracket")) {
    return { _tag: "MkPatR", _0: { _tag: "PList", _0: acc }, _1: tl(tokens) };
  } else {
    if (tok_is(hd(tokens), "dotdot")) {
      return { _tag: "MkPatR", _0: { _tag: "PList", _0: acc }, _1: tl(tl(tl(tokens))) };
    } else {
      const pr = parse_pattern(tokens);
      switch (pr._tag) {
        case "MkPatR": {
          const pat = pr._0;
          const rest = pr._1;
          if (tok_is(hd(rest), "comma")) {
            return parse_list_pattern_cont(tl(rest), append(acc, [pat]));
          } else {
            return tok_is(hd(rest), "rbracket") ? { _tag: "MkPatR", _0: { _tag: "PList", _0: append(acc, [pat]) }, _1: tl(rest) } : { _tag: "MkPatR", _0: { _tag: "PList", _0: append(acc, [pat]) }, _1: rest };
          }
          break;
        }
      }
    }
  }
}

function parse_if(tokens) {
  const cr = parse_expr(tokens);
  switch (cr._tag) {
    case "MkPR": {
      const cond = cr._0;
      const rest = cr._1;
      if (tok_is(hd(rest), "lbrace")) {
        const tr = parse_block_body(tl(rest));
        switch (tr._tag) {
          case "MkPR": {
            const then_expr = tr._0;
            const rest2 = tr._1;
            if (tok_is(hd(rest2), "keyword")) {
              if (tok_keyword_val(hd(rest2)) === "else") {
                if (tok_is(hd2(rest2), "lbrace")) {
                  const er = parse_block_body(tl2(rest2));
                  switch (er._tag) {
                    case "MkPR": {
                      const else_expr = er._0;
                      const rest3 = er._1;
                      return { _tag: "MkPR", _0: { _tag: "EIf", _0: cond, _1: then_expr, _2: else_expr }, _1: rest3 };
                      break;
                    }
                  }
                } else {
                  if (tok_is(hd2(rest2), "keyword")) {
                    if (tok_keyword_val(hd2(rest2)) === "if") {
                      const er = parse_if(tl2(rest2));
                      switch (er._tag) {
                        case "MkPR": {
                          const elif_expr = er._0;
                          const rest3 = er._1;
                          return { _tag: "MkPR", _0: { _tag: "EIf", _0: cond, _1: then_expr, _2: elif_expr }, _1: rest3 };
                          break;
                        }
                      }
                    } else {
                      return { _tag: "MkPR", _0: { _tag: "EIf", _0: cond, _1: then_expr, _2: { _tag: "EVar", _0: "undefined" } }, _1: rest2 };
                    }
                  } else {
                    return { _tag: "MkPR", _0: { _tag: "EIf", _0: cond, _1: then_expr, _2: { _tag: "EVar", _0: "undefined" } }, _1: rest2 };
                  }
                }
              } else {
                return { _tag: "MkPR", _0: { _tag: "EIf", _0: cond, _1: then_expr, _2: { _tag: "EVar", _0: "undefined" } }, _1: rest2 };
              }
            } else {
              return { _tag: "MkPR", _0: { _tag: "EIf", _0: cond, _1: then_expr, _2: { _tag: "EVar", _0: "undefined" } }, _1: rest2 };
            }
            break;
          }
        }
      } else {
        return { _tag: "MkPR", _0: { _tag: "EVar", _0: "_error" }, _1: rest };
      }
      break;
    }
  }
}

function parse_lambda(tokens) {
  const pr = parse_lambda_params(tokens);
  switch (pr._tag) {
    case "MkSLR": {
      const params = pr._0;
      const rest = pr._1;
      if (tok_is(hd(rest), "lbrace")) {
        const br = parse_block_body(tl(rest));
        switch (br._tag) {
          case "MkPR": {
            const body = br._0;
            const rem = br._1;
            return { _tag: "MkPR", _0: { _tag: "ELambda", _0: params, _1: body }, _1: rem };
            break;
          }
        }
      } else {
        const br = parse_expr(rest);
        switch (br._tag) {
          case "MkPR": {
            const body = br._0;
            const rem = br._1;
            return { _tag: "MkPR", _0: { _tag: "ELambda", _0: params, _1: body }, _1: rem };
            break;
          }
        }
      }
      break;
    }
  }
}

function parse_lambda_params(tokens) {
  if (tok_is(hd(tokens), "rparen")) {
    return { _tag: "MkSLR", _0: [], _1: tl(tokens) };
  } else {
    const name = tok_ident_val(hd(tokens));
    const second = hd2(tokens);
    if (second._tag === "TkComma") {
      const next = parse_lambda_params(tl2(tokens));
      switch (next._tag) {
        case "MkSLR": {
          const params = next._0;
          const rem = next._1;
          return { _tag: "MkSLR", _0: cons(name, params), _1: rem };
          break;
        }
      }
    } else if (second._tag === "TkRParen") {
      return { _tag: "MkSLR", _0: [name], _1: tl2(tokens) };
    } else if (second._tag === "TkColon") {
      const rest2 = skip_type_annotation(tl2(tokens));
      if (tok_is(hd(rest2), "comma")) {
        const next = parse_lambda_params(tl(rest2));
        switch (next._tag) {
          case "MkSLR": {
            const params = next._0;
            const rem = next._1;
            return { _tag: "MkSLR", _0: cons(name, params), _1: rem };
            break;
          }
        }
      } else {
        return tok_is(hd(rest2), "rparen") ? { _tag: "MkSLR", _0: [name], _1: tl(rest2) } : { _tag: "MkSLR", _0: [name], _1: rest2 };
      }
    } else {
      return { _tag: "MkSLR", _0: [], _1: tokens };
    }
  }
}

function parse_record_or_block(tokens) {
  if (tok_is(hd(tokens), "rbrace")) {
    return { _tag: "MkPR", _0: { _tag: "ERecord", _0: [] }, _1: tl(tokens) };
  } else {
    if (tok_is(hd(tokens), "ident")) {
      if (tok_is(hd2(tokens), "colon")) {
        const name = tok_ident_val(hd(tokens));
        const vr = parse_expr(tl2(tokens));
        switch (vr._tag) {
          case "MkPR": {
            const value = vr._0;
            const rest = vr._1;
            const fr = parse_record_fields(rest, [{ _tag: "MkField", _0: name, _1: value }]);
            switch (fr._tag) {
              case "MkLPR": {
                const fields = fr._0;
                const rem = fr._1;
                return { _tag: "MkPR", _0: { _tag: "ERecord", _0: fields }, _1: rem };
                break;
              }
            }
            break;
          }
        }
      } else {
        return parse_block_body(tokens);
      }
    } else {
      return parse_block_body(tokens);
    }
  }
}

function parse_record_fields(tokens, acc) {
  if (tok_is(hd(tokens), "rbrace")) {
    return { _tag: "MkLPR", _0: acc, _1: tl(tokens) };
  } else {
    if (tok_is(hd(tokens), "comma")) {
      if (tok_is(hd2(tokens), "rbrace")) {
        return { _tag: "MkLPR", _0: acc, _1: tl2(tokens) };
      } else {
        if (tok_is(hd2(tokens), "ident")) {
          if (tok_is(hd3(tokens), "colon")) {
            const name = tok_ident_val(hd2(tokens));
            const vr = parse_expr(tl3(tokens));
            switch (vr._tag) {
              case "MkPR": {
                const value = vr._0;
                const rest = vr._1;
                return parse_record_fields(rest, append(acc, [{ _tag: "MkField", _0: name, _1: value }]));
                break;
              }
            }
          } else {
            return { _tag: "MkLPR", _0: acc, _1: tokens };
          }
        } else {
          return { _tag: "MkLPR", _0: acc, _1: tokens };
        }
      }
    } else {
      return { _tag: "MkLPR", _0: acc, _1: tokens };
    }
  }
}

function parse_list_expr(tokens) {
  if (tok_is(hd(tokens), "rbracket")) {
    return { _tag: "MkPR", _0: { _tag: "EList", _0: [] }, _1: tl(tokens) };
  } else {
    const er = parse_list_elements(tokens);
    switch (er._tag) {
      case "MkLPR": {
        const elems = er._0;
        const rest = er._1;
        return { _tag: "MkPR", _0: { _tag: "EList", _0: elems }, _1: rest };
        break;
      }
    }
  }
}

function parse_list_elements(tokens) {
  const er = parse_expr(tokens);
  switch (er._tag) {
    case "MkPR": {
      const expr = er._0;
      const rest = er._1;
      if (tok_is(hd(rest), "comma")) {
        const next = parse_list_elements(tl(rest));
        switch (next._tag) {
          case "MkLPR": {
            const exprs = next._0;
            const rem = next._1;
            return { _tag: "MkLPR", _0: cons(expr, exprs), _1: rem };
            break;
          }
        }
      } else {
        return tok_is(hd(rest), "rbracket") ? { _tag: "MkLPR", _0: [expr], _1: tl(rest) } : { _tag: "MkLPR", _0: [expr], _1: rest };
      }
      break;
    }
  }
}

function emit_module(decls) {
  return join(map_decls(decls), "\n\n") + "\n";
}

function map_decls(decls) {
  if (decls.length === 0) {
    return [];
  } else if (decls.length >= 1) {
    const d = decls[0];
    const rest = decls.slice(1);
    return cons(emit_decl(d), map_decls(rest));
  }
}

function emit_decl(decl) {
  switch (decl._tag) {
    case "DFn": {
      const name = decl._0;
      const params = decl._1;
      const body = decl._2;
      return "function " + name + "(" + join(params, ", ") + ") {\n" + "  return " + emit_expr(body) + ";\n" + "}";
      break;
    }
    case "DPubFn": {
      const name = decl._0;
      const params = decl._1;
      const body = decl._2;
      return "export function " + name + "(" + join(params, ", ") + ") {\n" + "  return " + emit_expr(body) + ";\n" + "}";
      break;
    }
    case "DType": {
      const name = decl._0;
      const variants = decl._1;
      return emit_type_decl(name, variants);
      break;
    }
  }
}

function emit_type_decl(name, variants) {
  const type_line = "type " + name + " = " + join(map_variant_types(variants), " | ") + ";";
  const ctors = join(map_variant_ctors(name, variants), "\n");
  return type_line + "\n" + ctors;
}

function map_variant_types(variants) {
  if (variants.length === 0) {
    return [];
  } else if (variants.length >= 1) {
    const v = variants[0];
    const rest = variants.slice(1);
    switch (v._tag) {
      case "MkVariant": {
        const name = v._0;
        const n = v._1;
        return n === 0 ? cons("{ _tag: \"" + name + "\" }", map_variant_types(rest)) : cons("{ _tag: \"" + name + "\"; " + emit_field_types(n, 0) + " }", map_variant_types(rest));
        break;
      }
    }
  }
}

function map_variant_ctors(type_name, variants) {
  if (variants.length === 0) {
    return [];
  } else if (variants.length >= 1) {
    const v = variants[0];
    const rest = variants.slice(1);
    switch (v._tag) {
      case "MkVariant": {
        const name = v._0;
        const n = v._1;
        return n === 0 ? cons("const " + name + ": " + type_name + " = { _tag: \"" + name + "\" };", map_variant_ctors(type_name, rest)) : cons("const " + name + " = (" + emit_field_params(n, 0) + "): " + type_name + " => ({ _tag: \"" + name + "\", " + emit_field_names(n, 0) + " });", map_variant_ctors(type_name, rest));
        break;
      }
    }
  }
}

function emit_field_types(total, i) {
  if (i >= total) {
    return "";
  } else {
    return i === total - 1 ? "_" + show(i) + ": unknown" : "_" + show(i) + ": unknown; " + emit_field_types(total, i + 1);
  }
}

function emit_field_params(total, i) {
  if (i >= total) {
    return "";
  } else {
    return i === total - 1 ? "_" + show(i) + ": unknown" : "_" + show(i) + ": unknown, " + emit_field_params(total, i + 1);
  }
}

function emit_field_names(total, i) {
  if (i >= total) {
    return "";
  } else {
    return i === total - 1 ? "_" + show(i) : "_" + show(i) + ", " + emit_field_names(total, i + 1);
  }
}

function emit_expr(expr) {
  switch (expr._tag) {
    case "EInt": {
      const n = expr._0;
      return n;
      break;
    }
    case "EFloat": {
      const n = expr._0;
      return n;
      break;
    }
    case "EString": {
      const s = expr._0;
      return "\"" + s + "\"";
      break;
    }
    case "EBool": {
      const b = expr._0;
      return b ? "true" : "false";
      break;
    }
    case "EVar": {
      const name = expr._0;
      return name;
      break;
    }
    case "EBinOp": {
      const op = expr._0;
      const left = expr._1;
      const right = expr._2;
      return emit_expr(left) + " " + emit_op(op) + " " + emit_expr(right);
      break;
    }
    case "EUnaryOp": {
      const op = expr._0;
      const operand = expr._1;
      return op + emit_expr(operand);
      break;
    }
    case "EConcat": {
      const a = expr._0;
      const b = expr._1;
      return emit_expr(a) + " + " + emit_expr(b);
      break;
    }
    case "EPipe": {
      const left = expr._0;
      const right = expr._1;
      return emit_expr(right) + "(" + emit_expr(left) + ")";
      break;
    }
    case "EApp": {
      const fn_expr = expr._0;
      const args = expr._1;
      return emit_expr(fn_expr) + "(" + join(map_exprs(args), ", ") + ")";
      break;
    }
    case "EConstruct": {
      const tag = expr._0;
      const args = expr._1;
      return emit_construct(tag, args);
      break;
    }
    case "ELambda": {
      const params = expr._0;
      const body = expr._1;
      return "(" + join(params, ", ") + ") => " + emit_expr(body);
      break;
    }
    case "ELet": {
      const name = expr._0;
      const value = expr._1;
      const body = expr._2;
      return "(() => {\n" + "  const " + name + " = " + emit_expr(value) + ";\n" + "  return " + emit_expr(body) + ";\n" + "})()";
      break;
    }
    case "EIf": {
      const cond = expr._0;
      const then_expr = expr._1;
      const else_expr = expr._2;
      return "(" + emit_expr(cond) + " ? " + emit_expr(then_expr) + " : " + emit_expr(else_expr) + ")";
      break;
    }
    case "EMatch": {
      const scrutinee = expr._0;
      const arms = expr._1;
      return emit_match(scrutinee, arms);
      break;
    }
    case "ERecord": {
      const fields = expr._0;
      return "{ " + join(map_fields(fields), ", ") + " }";
      break;
    }
    case "EFieldAccess": {
      const obj = expr._0;
      const field = expr._1;
      return emit_expr(obj) + "." + field;
      break;
    }
    case "EList": {
      const elems = expr._0;
      return "[" + join(map_exprs(elems), ", ") + "]";
      break;
    }
    case "EBlock": {
      const exprs = expr._0;
      return emit_block(exprs);
      break;
    }
  }
}

function emit_op(op) {
  if (op === "==") {
    return "===";
  } else {
    return op === "!=" ? "!==" : op;
  }
}

function map_exprs(exprs) {
  if (exprs.length === 0) {
    return [];
  } else if (exprs.length >= 1) {
    const e = exprs[0];
    const rest = exprs.slice(1);
    return cons(emit_expr(e), map_exprs(rest));
  }
}

function map_fields(fields) {
  if (fields.length === 0) {
    return [];
  } else if (fields.length >= 1) {
    const f = fields[0];
    const rest = fields.slice(1);
    switch (f._tag) {
      case "MkField": {
        const name = f._0;
        const value = f._1;
        return cons(name + ": " + emit_expr(value), map_fields(rest));
        break;
      }
    }
  }
}

function emit_construct(tag, args) {
  if (args.length === 0) {
    return "{ _tag: \"" + tag + "\" }";
  } else {
    return "{ _tag: \"" + tag + "\", " + emit_construct_fields(args, 0) + " }";
  }
}

function emit_construct_fields(args, i) {
  if (args.length === 0) {
    return "";
  } else if (args.length >= 1) {
    const arg = args[0];
    const rest = args.slice(1);
    if (rest.length === 0) {
      return "_" + show(i) + ": " + emit_expr(arg);
    } else {
      return "_" + show(i) + ": " + emit_expr(arg) + ", " + emit_construct_fields(rest, i + 1);
    }
  }
}

function emit_match(scrutinee, arms) {
  const scr = emit_expr(scrutinee);
  return all_ctor_arms(arms) ? emit_tag_switch(scr, arms) : emit_if_chain(scr, arms);
}

function all_ctor_arms(arms) {
  if (arms.length === 0) {
    return true;
  } else if (arms.length >= 1) {
    const a = arms[0];
    const rest = arms.slice(1);
    return is_ctor_arm(a) ? all_ctor_arms(rest) : false;
  }
}

function is_ctor_arm(arm) {
  switch (arm._tag) {
    case "MkArm": {
      const pat = arm._0;
      if (pat._tag === "PConstructor") {
        return true;
      } else {
        return false;
      }
      break;
    }
  }
}

function emit_tag_switch(scr, arms) {
  return "(() => {\n" + "  switch (" + scr + "._tag) {\n" + join(map_switch_arms(scr, arms), "\n") + "\n" + "  }\n" + "})()";
}

function map_switch_arms(scr, arms) {
  if (arms.length === 0) {
    return [];
  } else if (arms.length >= 1) {
    const a = arms[0];
    const rest = arms.slice(1);
    return cons(emit_one_switch_arm(scr, a), map_switch_arms(scr, rest));
  }
}

function emit_one_switch_arm(scr, arm) {
  switch (arm._tag) {
    case "MkArm": {
      const pat = arm._0;
      const body = arm._1;
      if (pat._tag === "PConstructor") {
        const tag = pat._0;
        const pats = pat._1;
        return "    case \"" + tag + "\": {\n" + emit_pat_bindings(pats, scr, 0) + "      return " + emit_expr(body) + ";\n" + "    }";
      } else {
        return "    default: {\n" + "      return " + emit_expr(body) + ";\n" + "    }";
      }
      break;
    }
  }
}

function emit_pat_bindings(pats, scr, i) {
  if (pats.length === 0) {
    return "";
  } else if (pats.length >= 1) {
    const p = pats[0];
    const rest = pats.slice(1);
    if (p._tag === "PVar") {
      const name = p._0;
      return "      const " + name + " = " + scr + "._" + show(i) + ";\n" + emit_pat_bindings(rest, scr, i + 1);
    } else {
      return emit_pat_bindings(rest, scr, i + 1);
    }
  }
}

function emit_if_chain(scr, arms) {
  return "(() => {\n" + "  const __scr = " + scr + ";\n" + emit_if_arms("__scr", arms, true) + "})()";
}

function emit_if_arms(scr, arms, is_first) {
  if (arms.length === 0) {
    return "";
  } else if (arms.length >= 1) {
    const a = arms[0];
    const rest = arms.slice(1);
    switch (a._tag) {
      case "MkArm": {
        const pat = a._0;
        const body = a._1;
        if (rest.length === 0) {
          return emit_last_arm(scr, pat, body, is_first);
        } else {
          return emit_mid_arm(scr, pat, body, rest, is_first);
        }
        break;
      }
    }
  }
}

function emit_last_arm(scr, pat, body, is_first) {
  const cond = emit_pat_cond(pat, scr);
  if (cond === "true") {
    return is_first ? "  {\n" + emit_arm_binds(pat, scr) + "    return " + emit_expr(body) + ";\n  }\n" : "  } else {\n" + emit_arm_binds(pat, scr) + "    return " + emit_expr(body) + ";\n  }\n";
  } else {
    const pfx = (is_first ? "  if (" : "  } else if (");
    return pfx + cond + ") {\n" + emit_arm_binds(pat, scr) + "    return " + emit_expr(body) + ";\n  }\n";
  }
}

function emit_mid_arm(scr, pat, body, rest, is_first) {
  const cond = emit_pat_cond(pat, scr);
  const pfx = (is_first ? "  if (" : "  } else if (");
  return pfx + cond + ") {\n" + emit_arm_binds(pat, scr) + "    return " + emit_expr(body) + ";\n" + emit_if_arms(scr, rest, false);
}

function emit_pat_cond(pat, scr) {
  switch (pat._tag) {
    case "PVar": {
      return "true";
      break;
    }
    case "PWildcard": {
      return "true";
      break;
    }
    case "PConstructor": {
      const tag = pat._0;
      return scr + "._tag === \"" + tag + "\"";
      break;
    }
    case "PLiteral": {
      const expr = pat._0;
      return scr + " === " + emit_expr(expr);
      break;
    }
    case "PList": {
      const elems = pat._0;
      if (elems.length === 0) {
        return scr + ".length === 0";
      } else {
        return scr + ".length >= " + show(pat_list_len(elems));
      }
      break;
    }
  }
}

function pat_list_len(pats) {
  if (pats.length === 0) {
    return 0;
  } else if (pats.length >= 1) {
    const rest = pats.slice(1);
    return 1 + pat_list_len(rest);
  }
}

function emit_arm_binds(pat, scr) {
  switch (pat._tag) {
    case "PVar": {
      const name = pat._0;
      return "    const " + name + " = " + scr + ";\n";
      break;
    }
    case "PConstructor": {
      const args = pat._1;
      return emit_ctor_binds(args, scr, 0);
      break;
    }
    case "PList": {
      const elems = pat._0;
      return emit_list_binds(elems, scr, 0);
      break;
    }
    case "PWildcard": {
      return "";
      break;
    }
    case "PLiteral": {
      return "";
      break;
    }
  }
}

function emit_ctor_binds(pats, scr, i) {
  if (pats.length === 0) {
    return "";
  } else if (pats.length >= 1) {
    const p = pats[0];
    const rest = pats.slice(1);
    if (p._tag === "PVar") {
      const name = p._0;
      return "    const " + name + " = " + scr + "._" + show(i) + ";\n" + emit_ctor_binds(rest, scr, i + 1);
    } else {
      return emit_ctor_binds(rest, scr, i + 1);
    }
  }
}

function emit_list_binds(pats, scr, i) {
  if (pats.length === 0) {
    return "";
  } else if (pats.length >= 1) {
    const p = pats[0];
    const rest = pats.slice(1);
    if (p._tag === "PVar") {
      const name = p._0;
      return "    const " + name + " = " + scr + "[" + show(i) + "];\n" + emit_list_binds(rest, scr, i + 1);
    } else {
      return emit_list_binds(rest, scr, i + 1);
    }
  }
}

function emit_block(exprs) {
  if (exprs.length === 0) {
    return "undefined";
  } else if (exprs.length >= 1) {
    const single = exprs[0];
    const more = exprs.slice(1);
    if (more.length === 0) {
      return emit_expr(single);
    } else {
      return "(() => {\n" + emit_block_stmts(exprs) + "})()";
    }
  }
}

function emit_block_stmts(exprs) {
  if (exprs.length === 0) {
    return "";
  } else if (exprs.length >= 1) {
    const expr = exprs[0];
    const rest = exprs.slice(1);
    if (rest.length === 0) {
      return emit_final_stmt(expr);
    } else {
      return emit_non_final_stmt(expr) + emit_block_stmts(rest);
    }
  }
}

function emit_final_stmt(expr) {
  if (expr._tag === "ELet") {
    const name = expr._0;
    const value = expr._1;
    const body = expr._2;
    return "  const " + name + " = " + emit_expr(value) + ";\n" + emit_final_stmt(body);
  } else {
    return "  return " + emit_expr(expr) + ";\n";
  }
}

function emit_non_final_stmt(expr) {
  if (expr._tag === "ELet") {
    const name = expr._0;
    const value = expr._1;
    const body = expr._2;
    return "  const " + name + " = " + emit_expr(value) + ";\n" + emit_non_final_stmt(body);
  } else {
    return "  " + emit_expr(expr) + ";\n";
  }
}

function main() {
  const source = read_file(get_arg(1));
  const tokens = tokenize(source);
  const decls = parse(tokens);
  const output = emit_module(decls);
  return println(output);
}

main();
