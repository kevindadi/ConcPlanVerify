"""Line- and column-preserving Rust comment and string/char literal stripper."""

from __future__ import annotations


def strip_rust_comments_and_literals(code: str) -> str:
    """Replace Rust comments and string/char literals with spaces, preserving newlines.

    Handles:
    - ``// ...`` line comments
    - nested ``/* ... */`` block comments
    - normal, byte, and C string literals (``"..."``, ``b"..."``, ``c"..."``)
    - raw string literals (``r"..."``, ``r#"..."#``, ``br#"..."#``, ``cr#"..."#``)
    - character and byte literals (``'a'``, ``'\\n'``, ``b'x'``) without consuming
      Rust lifetimes such as ``'a`` or ``'static``
    """
    n = len(code)
    out: list[str] = []
    i = 0

    def _emit_spaces(start: int, end: int) -> None:
        for ch in code[start:end]:
            out.append("\n" if ch == "\n" else " ")

    while i < n:
        ch = code[i]

        if ch == "/" and i + 1 < n and code[i + 1] == "/":
            j = i + 2
            while j < n and code[j] != "\n":
                j += 1
            _emit_spaces(i, j)
            i = j
            continue

        if ch == "/" and i + 1 < n and code[i + 1] == "*":
            depth = 1
            j = i + 2
            while j < n and depth > 0:
                if code[j] == "/" and j + 1 < n and code[j + 1] == "*":
                    depth += 1
                    j += 2
                elif code[j] == "*" and j + 1 < n and code[j + 1] == "/":
                    depth -= 1
                    j += 2
                else:
                    j += 1
            _emit_spaces(i, j)
            i = j
            continue

        raw_match = _match_raw_string_prefix(code, i)
        if raw_match is not None:
            hashes, content_start = raw_match
            closing = '"' + ("#" * hashes)
            end_idx = code.find(closing, content_start)
            j = n if end_idx == -1 else end_idx + len(closing)
            _emit_spaces(i, j)
            i = j
            continue

        if ch in ("b", "c") and i + 1 < n and code[i + 1] == '"' and (
            i == 0 or not (code[i - 1].isalnum() or code[i - 1] == "_")
        ):
            j = _scan_quoted_string(code, i + 1)
            _emit_spaces(i, j)
            i = j
            continue

        if ch == '"':
            j = _scan_quoted_string(code, i)
            _emit_spaces(i, j)
            i = j
            continue

        if ch == "b" and i + 1 < n and code[i + 1] == "'" and (
            i == 0 or not (code[i - 1].isalnum() or code[i - 1] == "_")
        ):
            end_char = _match_char_literal(code, i + 1)
            if end_char is not None:
                _emit_spaces(i, end_char)
                i = end_char
                continue

        if ch == "'":
            end_char = _match_char_literal(code, i)
            if end_char is not None:
                _emit_spaces(i, end_char)
                i = end_char
                continue

        out.append(ch)
        i += 1

    return "".join(out)


def _match_raw_string_prefix(code: str, i: int) -> tuple[int, int] | None:
    n = len(code)
    if i > 0 and (code[i - 1].isalnum() or code[i - 1] == "_"):
        return None
    j = i
    if j < n and code[j] in ("b", "c"):
        j += 1
    if j >= n or code[j] != "r":
        return None
    j += 1
    hashes = 0
    while j < n and code[j] == "#":
        hashes += 1
        j += 1
    if j < n and code[j] == '"':
        return hashes, j + 1
    return None


def _scan_quoted_string(code: str, quote_idx: int) -> int:
    n = len(code)
    j = quote_idx + 1
    while j < n:
        c = code[j]
        if c == "\\":
            j += 2
            continue
        if c == '"':
            return j + 1
        j += 1
    return n


def _match_char_literal(code: str, quote_idx: int) -> int | None:
    """Match a Rust char literal starting at ``quote_idx`` without matching lifetimes."""
    n = len(code)
    if quote_idx + 2 >= n:
        return None
    if code[quote_idx + 1] == "\\":
        j = quote_idx + 2
        if j >= n:
            return None
        if code[j] == "u" and j + 1 < n and code[j + 1] == "{":
            close_brace = code.find("}", j + 2)
            if close_brace != -1 and close_brace + 1 < n and code[close_brace + 1] == "'":
                return close_brace + 2
            return None
        if code[j] == "x":
            if j + 3 < n and code[j + 3] == "'":
                return j + 4
            return None
        if j + 1 < n and code[j + 1] == "'":
            return j + 2
        return None
    if code[quote_idx + 1] not in ("'", "\n") and code[quote_idx + 2] == "'":
        return quote_idx + 3
    return None
