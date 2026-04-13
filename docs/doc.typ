#import "@preview/shadowed:0.3.0": shadow

#let doc(code, vertical: false, width: 8cm) = {
  let dark = sys.inputs.at("dark", default: "false") == "true"
  let public-code = raw(
    block: true,
    lang: "typst",
    code.text.replace("@local/bulb", "@preview/bulb"),
  )

  let gridargs = arguments(columns: (width, width))
  if vertical {
    gridargs = arguments(columns: (1fr,), rows: (auto, auto))
  }

  let code_bg = white
  let shadow_clr = black
  let text_clr = black
  if dark {
    code_bg = rgb("#13181F")
    shadow_clr = white
    text_clr = white
  }

  set text(fill: text_clr)

  show raw.where(block: true): set block(fill: code_bg, inset: 1em, radius: .5em, width: 100%)

  let g = grid(
    ..gridargs,
    column-gutter: 1em,
    row-gutter: 1em,
    shadow(blur: 3pt, radius: 5pt, fill: shadow_clr.transparentize(85%), block(inset: 1pt, public-code)),
    block(
      fill: rgb("#CACBD1"),
      inset: 5pt,
      radius: 5pt,
      block(
        fill: white,
        inset: 1em,
        {
          set text(size: 6pt, fill: black)
          show image: set image(width: width - 1cm)
          eval(
            code.text,
            mode: "markup",
          )
        },
      ),
    ),
  )

  if vertical {
    block(width: width, g)
  } else {
    g
  }
}

#let document(content) = {
  set page(width: auto, height: auto, margin: 1em, fill: white.transparentize(100%))
  set text(size: 9pt)

  content
}
