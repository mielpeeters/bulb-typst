#import "doc.typ": doc, document

#show: document.with()

#doc(
  ```typ
  #import "@local/bulb:0.2.0": dither

  #figure(
    image(
      dither(
        read("tent.png", encoding: none),
        size: 500,
        palette: (green, rgb("#c1b38f"), "#1c2500", oklch(90%, 30%, 230deg)),
        method: "cluster4",
      ),
    ),
    caption: "User-defined palette",
  )
  ```,
)
