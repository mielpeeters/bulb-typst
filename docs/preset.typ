#import "doc.typ": doc, document

#show: document.with()

#doc(
  ```typ
  #import "@local/bulb:0.2.1": dither

  #figure(
    image(
      dither(
        read("koln.jpg", encoding: none),
        size: 500,
        method: "bayer8",
        palette: "pico8",
        gamma: 1.5,
      ),
    ),
    caption: "bayer8x8 with Pico8 colours",
  )
  ```,
)
