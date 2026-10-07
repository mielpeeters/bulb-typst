#import "doc.typ": doc, document

#show: document.with()


#doc(
  ```typ
  #import "@local/bulb:0.4.0": dither

  #grid(
    columns: (1fr,),
    row-gutter: 1em,
    figure(
      image("flowers.jpg", width: 100%),
      caption: "Original"
    ),
    figure(
      image(
        dither(
          path("flowers.jpg"),
          size: 400,
          colors: 16,
          method: "floyd",
          hull-weight: 500,
        ),
        width: 100%
      ),
      caption: [High importance on \ Convex Hull coverage],
    ),
    figure(
      image(
        dither(
          path("flowers.jpg"),
          size: 400,
          colors: 16,
          method: "floyd",
          hull-weight: 0,
        ),
        width: 100%
      ),
      caption: [High importance on \ Low Noise output],
    )
  )
  ```,
  // vertical: true,
  width: 8cm,
)
