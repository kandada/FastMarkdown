use fastmarkdown::*;
fn main() {
    let md = "# 标题\n\n一段 **粗体** 和 `code`\n\n```bash\necho hello\nls -la\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n- item1\n- item2\n";
    let out = to_spans(md);
    let text = unsafe { std::slice::from_raw_parts(out.text, out.text_len as usize) };
    let text = String::from_utf8_lossy(text);
    let spans = unsafe { std::slice::from_raw_parts(out.spans, out.span_count as usize) };
    let extra = unsafe { std::slice::from_raw_parts(out.extra_data, out.extra_data_len as usize) };
    println!("TEXT({}): {:?}", text.len(), text);
    println!("EXTRA({}): {:?}", extra.len(), String::from_utf8_lossy(extra));
    for s in spans {
        let sub = String::from_utf8_lossy(&text.as_bytes()[s.offset as usize..(s.offset + s.length) as usize]);
        println!("  off={:4} len={:4} flags={:#08x} blk={:2} depth={} seq={:3} extra_len={} text={:?}",
            s.offset, s.length, s.flags, s.block_type, s.block_depth, s.block_seq, s.extra_len, sub);
    }
    unsafe { free_spans(out) };
}
