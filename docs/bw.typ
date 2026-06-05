#import "doc.typ": doc, document

#show: document.with()


#doc(
  ```typ
  #import "@local/bulb:0.2.0": dither

  #figure(
    image(
      dither(
        read("bromo.png", encoding: none),
        size: 800,
        mode: "bw",
        method: "cluster8",
        contrast: 1.5,
      ),
    ),
    caption: "Clustered-dot dithering matrix in B/W",
  )
  ```,
)
