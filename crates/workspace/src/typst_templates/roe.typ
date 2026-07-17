#let roe(findings: (), requirements: (), scope: none, sections: (:), metadata: (:), notes: (), doc) = {
  set page(paper: "a4", margin: 2cm)
  set text(font: "DejaVu Sans", 11pt)
  set heading(numbering: "1.")

  #text(size: 20pt, weight: "bold")[Rules of Engagement]
  #v(0.5cm)
  #metadata.at("client_name", default: "")
  #v(0.5cm)

  doc
}