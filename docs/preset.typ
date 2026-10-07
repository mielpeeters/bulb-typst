#import "doc.typ": doc, document

#show: document.with()

#doc(
  ```typ
  #import "@local/bulb:0.4.0": dither

  #figure(
    image(
      dither(
        path("koln.jpg"),
        size: 500,
        method: "bayer8",
        colors: "pico8",
        gamma: 1.5,
      ),
    ),
    caption: "Bayer8 with Pico8 colours",
  )
  ```,
)
