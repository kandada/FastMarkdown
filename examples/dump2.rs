use fastmarkdown::*;
fn main() {
    let md = "> 引用 *斜体*\n\n---\n";
    let out = to_spans(md);
    let text = unsafe { std::slice::from_raw_parts(out.text, out.text_len as usize) };
    let text = String::from_utf8_lossy(text);
    let spans = unsafe { std::slice::from_raw_parts(out.spans, out.span_count as usize) };
    println!("TEXT: {:?}", text);
    for s in spans {
        let sub = String::from_utf8_lossy(
            &text.as_bytes()[s.offset as usize..(s.offset + s.length) as usize],
        );
        println!(
            "  blk={:2} seq={:3} depth={} text={:?}",
            s.block_type, s.block_seq, s.block_depth, sub
        );
    }
    unsafe { free_spans(out) };
}
