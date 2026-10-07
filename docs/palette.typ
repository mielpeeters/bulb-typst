#import "doc.typ": doc, document

#show: document.with()


#doc(
  ```typ
  #import "@local/bulb:0.4.0": dither

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
          path("koln.jpg"),
          size: 200,
          method: "floyd",
          colors: 12,
        ),
        width: 100%
      ),
      caption: [Floyd-Steinberg diffusion dithering \ with *generated* palette],
    )
  )
  ```,
  // vertical: true,
  width: 8cm,
)
