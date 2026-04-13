#import "doc.typ": doc, document

#show: document.with()

#doc(
  ```typ
  #import "@local/bulb:0.1.0": dither

  #figure(
    image(
      dither(
        read("bromo.png", encoding: none),
        size: 500,
        levels: 4,
        mode: "rgb",
        method: "bayer8",
      ),
    ),
    caption: "bayer8x8 RGB channels, each with 4 levels",
  )
  ```,
)
