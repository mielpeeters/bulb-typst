#import "doc.typ": doc, document

#show: document.with()


#doc(
  ```typ
  #import "@local/bulb:0.3.0": dither

  #figure(
    image(
      dither(
        path("bromo.png"),
        size: 800,
        colors: "bw",
        method: "cluster8",
        contrast: 1.5,
      ),
    ),
    caption: "Clustered-dot dithering matrix in B/W",
  )
  ```,
)
