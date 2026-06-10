#import "doc.typ": doc, document

#show: document.with()

#doc(
  ```typ
  #import "@local/bulb:0.2.1": dither

  #figure(
    image(
      dither(
        read("tent.png", encoding: none),
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
