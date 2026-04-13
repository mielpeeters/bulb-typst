#import "doc.typ": doc, document

#show: document.with()


#doc(
  ```typ
  #import "@local/bulb:0.1.0": dither

  #grid(
    columns: (1fr, 1fr),
    column-gutter: 1em,
    figure(
      image("koln.jpg", width: 100%),
      caption: "Original"
    ),
    figure(
      image(
        dither(
          read("koln.jpg", encoding: none),
          size: 200,
          mode: "palette",
          method: "bayer4",
          colors: 15,
        ),
        // better results in pngs, svgs
        scaling: "pixelated",
        width: 100%
      ),
      caption: [Bayer8x8 dithering matrix \ with *generated* palette],
    )
  )
  ```,
  // vertical: true,
  width: 8cm,
)
