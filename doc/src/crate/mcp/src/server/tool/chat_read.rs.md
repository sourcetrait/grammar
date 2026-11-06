# chat_read.rs

`interface chat read <id> <page?>`: a chat's `<id>/message.md` in the open
Channel's inbox, a page at a time, answering `{id, page, pages, text}`.

## fn ChannelFace::read_chat
The id is held to base62 before it joins the inbox, so no id reaches outside
its packet's `<id>/`; the inbox is the open Channel's, and a closed Channel is
`channel::not_open`. The file is read off the runtime's workers. A page past the
last is refused, naming the count.

## fn page_bounds
A page is at most 4 KiB, under a tool result's comfortable size, and is cut
after its last line break so no line splits; a page holding none is cut at the
last character boundary within it. An empty message is one empty page, so every
chat reads at page 0.
