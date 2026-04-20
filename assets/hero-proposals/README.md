# Hero proposals — pick one

Five candidates for the top-of-README banner. None of them is live yet — the
README currently points at `assets/banner.svg`. When you decide, I'll swap it
in.

| File | Vibe | Best for |
|------|------|----------|
| [`A-minimal-metric.svg`](A-minimal-metric.svg) | Current style, polished. Single hero metric `22 µs` on the left, four secondary metrics stacked right. Monochrome black + rust. | Engineers. Feels like Linear/Railway. |
| [`B-giant-type.svg`](B-giant-type.svg) | Poster-size `ONE FILE. FULL BRAIN.` typography. Footnote chip with numbers. | HN front page. High contrast screenshot. |
| [`C-terminal.svg`](C-terminal.svg) | Fake terminal window showing `put → search → snap → scp → verify`. Each line is real CLI output. | Devs who trust code more than adjectives. |
| [`D-synaptic-network.svg`](D-synaptic-network.svg) | Abstract neural-network visual on the right, giant metrics on the left. | Thought-leadership vibe, slide decks. |
| [`E-stripe-style.svg`](E-stripe-style.svg) | Off-white background, five metric cards, pipeline chip. Stripe/Linear energy. | Landing pages, light-mode preference. |

## How to switch the hero

Replace the first line in `README.md` after the `<div align="center">` with
whichever file you like:

```md
<img src="assets/hero-proposals/A-minimal-metric.svg" alt="…" width="100%"/>
```

Or tell me which (A/B/C/D/E) and I'll patch + push.

## Full-size comparison matrix

`assets/matrix-full.svg` is the bigger, more readable version of the capability
matrix — 1600×1200, larger type, wider column gaps, Synapse row at the top with
the rust border treatment. Referenced from the README via the "Compared to the
field" section.
