# Lyrebird

This is an example of using `jungle` to perform optimization of some audio synthesis code using LLMs and [Monte Carlo Tree Search](https://en.wikipedia.org/wiki/Monte_Carlo_tree_search).

<img width="1281" height="660" alt="Image" src="https://github.com/user-attachments/assets/96343a9b-89b1-4ab8-82e7-d802a16e07a4" />

Click the spectrograms to play the corresponding audio.

Run it with:

```
cargo run --example lyrebird --release
```

I haven't devoted much compute to this yet, but you can try using `--system-prompt-override` to see how injecting different context impacts the search results.

