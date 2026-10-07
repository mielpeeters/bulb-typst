#import "doc.typ": doc, document

#show: document.with()

#doc(
  ```typ
  #import "@local/bulb:0.4.0": dither

  #figure(
    image(
      dither(
        path("tent.png"),
        size: 500,
        colors: (
          white, rgb("#a1a14d"),
          rgb("#6da9ee"), "#4d1515",
          oklch(90%, 50%, 30deg)
        ),
        method: "cluster4",
      ),
    ),
    caption: "User-defined palette",
  )
  ```,
)
