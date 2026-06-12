/// Insere char na posição do cursor e avança o cursor.
pub(super) fn edit_cursor_insert(buf: &mut String, cursor: &mut usize, c: char) {
    let byte = buf.char_indices().nth(*cursor).map(|(i, _)| i).unwrap_or(buf.len());
    buf.insert(byte, c);
    *cursor += 1;
}

/// Apaga o char antes do cursor (Backspace).
pub(super) fn edit_cursor_backspace(buf: &mut String, cursor: &mut usize) {
    if *cursor > 0 {
        let byte = buf.char_indices().nth(*cursor - 1).map(|(i, _)| i).unwrap_or(0);
        buf.remove(byte);
        *cursor -= 1;
    }
}

/// Apaga o char na posição do cursor (Delete).
pub(super) fn edit_cursor_delete(buf: &mut String, cursor: usize) {
    if let Some((byte, _)) = buf.char_indices().nth(cursor) {
        buf.remove(byte);
    }
}

/// Produz a string de exibição com █ na posição do cursor.
pub(super) fn edit_render_cursor(buf: &str, cursor: usize) -> String {
    let chars: Vec<char> = buf.chars().collect();
    let cur = cursor.min(chars.len());
    let before: String = chars[..cur].iter().collect();
    let after: String = chars[cur..].iter().collect();
    format!("{before}█{after}")
}

// ── Navegação por linha no buffer de edição ───────────────────────────────────
// O separador de linha é o literal dois-chars "\n" (backslash + n).

/// Retorna os índices de char onde cada linha começa.
pub(super) fn edit_line_starts(buf: &str) -> Vec<usize> {
    let chars: Vec<char> = buf.chars().collect();
    let mut starts = vec![0usize];
    let mut i = 0;
    while i + 1 < chars.len() {
        if chars[i] == '\\' && chars[i + 1] == 'n' {
            starts.push(i + 2);
            i += 2;
        } else {
            i += 1;
        }
    }
    starts
}

/// Retorna (índice_da_linha, coluna) para a posição de cursor dada.
fn edit_line_col(buf: &str, cursor: usize) -> (usize, usize) {
    let starts = edit_line_starts(buf);
    let total = buf.chars().count();
    let cursor = cursor.min(total);
    for i in (0..starts.len()).rev() {
        if cursor >= starts[i] {
            let line_end = if i + 1 < starts.len() {
                starts[i + 1].saturating_sub(2)
            } else {
                total
            };
            let col = cursor.min(line_end) - starts[i];
            return (i, col);
        }
    }
    (0, 0)
}

/// Converte (linha, coluna) de volta para posição de cursor.
fn edit_pos_from_line_col(buf: &str, line: usize, col: usize) -> usize {
    let starts = edit_line_starts(buf);
    let total = buf.chars().count();
    let line_start = starts.get(line).copied().unwrap_or(total);
    let line_end = if line + 1 < starts.len() {
        starts[line + 1].saturating_sub(2)
    } else {
        total
    };
    (line_start + col).min(line_end)
}

/// Move o cursor uma linha acima, mantendo a coluna.
pub(super) fn edit_cursor_up(buf: &str, cursor: usize) -> usize {
    let (line, col) = edit_line_col(buf, cursor);
    if line == 0 { return 0; }
    edit_pos_from_line_col(buf, line - 1, col)
}

/// Move o cursor uma linha abaixo, mantendo a coluna.
pub(super) fn edit_cursor_down(buf: &str, cursor: usize) -> usize {
    let starts = edit_line_starts(buf);
    let (line, col) = edit_line_col(buf, cursor);
    if line + 1 >= starts.len() { return buf.chars().count(); }
    edit_pos_from_line_col(buf, line + 1, col)
}

/// Início da linha atual.
pub(super) fn edit_line_start(buf: &str, cursor: usize) -> usize {
    let (line, _) = edit_line_col(buf, cursor);
    edit_line_starts(buf).get(line).copied().unwrap_or(0)
}

/// Final da linha atual (antes do separador \n ou fim do buffer).
pub(super) fn edit_line_end(buf: &str, cursor: usize) -> usize {
    let starts = edit_line_starts(buf);
    let (line, _) = edit_line_col(buf, cursor);
    let total = buf.chars().count();
    if line + 1 < starts.len() {
        starts[line + 1].saturating_sub(2)
    } else {
        total
    }
}
