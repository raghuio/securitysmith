#let report(findings: (), requirements: (), scope: none, sections: (:), metadata: (:), notes: (), doc) = {
  set page(paper: "a4", margin: 2cm)
  set text(font: "DejaVu Sans", 11pt)
  set heading(numbering: "1.")

  // Cover
  align(center)[
    #text(size: 24pt, weight: "bold")[Security Assessment Report]
    #v(0.5cm)
    #text(size: 14pt)[
      #metadata.at("client_name", default: "") — #metadata.at("project_name", default: "")
    ]
    #v(0.3cm)
    #text(size: 11pt)[
      #metadata.at("engagement_name", default: "")
      #if metadata.at("start_date", default: "") != "" [ (#metadata.at("start_date", default: "") — #metadata.at("end_date", default: ""))]
    ]
  ]
  pagebreak()

  // Scope
  if scope != none [
    = Scope
    #scope
    pagebreak()
  ]

  // Sections (methodology, mitigations, etc.)
  for (name, content) in sections {
    if content != none [
      = #name.replace("_", " ")
      #content
      pagebreak()
    ]
  }

  // Findings
  if findings.len() > 0 [
    = Findings (#findings.len())

    #for f in findings [
      === #f.id — #f.severity
      *Status:* #f.status
      #v(0.3cm)
      #f.body
      pagebreak()
    ]
  ]

  doc
}