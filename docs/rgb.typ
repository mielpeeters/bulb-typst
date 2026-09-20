#import "doc.typ": doc, document

#show: document.with()

#doc(
  ```typ
  #import "@local/bulb:0.3.0": dither

  #figure(
    image(
      dither(
        path("tent.png"),
        size: 500,
        levels: 4,
        colors: "rgb",
        method: "bayer8",
      ),
    ),
    caption: "bayer8x8 RGB channels, each with 4 levels",
  )
  ```,
)
