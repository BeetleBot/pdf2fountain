use pdf2fountain::parse_pdf_file_to_fountain;
use std::path::Path;

#[test]
fn test_testfile_pdf() {
    let pdf_path = "TestingFiles/testfile.pdf";
    if !Path::new(pdf_path).exists() {
        return;
    }

    let result = parse_pdf_file_to_fountain(pdf_path).expect("failed parsing testfile.pdf");
    assert!(result.contains("Title: TEST FILE"));
    assert!(result.contains("Author: Nirmal"));
    assert!(result.contains(".INT. MEDICAL COLLEGE - CORRIDOR - DAY #2#"));
    assert!(result.contains("@VIKRUTI"));
    assert!(result.contains("@ARJUN"));
    assert!(result.contains("@VINOTH"));
    assert!(result.contains("@MADHU"));
    assert!(result.contains("student-ah thirumba paakka koodadhu."));
    assert!(result.contains("> CUT TO:"));
}

#[test]
fn test_sample_a4_output() {
    let pdf_path = "TestingFiles/sample_A4.pdf";
    if !Path::new(pdf_path).exists() {
        return;
    }

    let result = parse_pdf_file_to_fountain(pdf_path).expect("failed parsing A4 pdf");
    assert!(result.contains("Title: THIS IS A TEST FILE"));
    assert!(result.to_uppercase().contains(".INT. GANESH'S HOUSE - DAY #10#"));
    assert!(result.contains("@REENA"));
    assert!(result.contains("@ROCKY"));
    assert!(result.contains("@SINDHU"));
    assert!(result.contains("!Reena closes the kitchen door and looks at rocky and sindhu"));
    assert!(result.contains("> CUT TO:"));
    assert!(!result.contains("(CONT'D)"));
    assert!(!result.contains("(MORE)"));
}

#[test]
fn test_sample_usletter_output() {
    let pdf_path = "TestingFiles/sample_USLETTER.pdf";
    if !Path::new(pdf_path).exists() {
        return;
    }

    let result = parse_pdf_file_to_fountain(pdf_path).expect("failed parsing US Letter pdf");
    assert!(result.contains("Title: THIS IS A TEST FILE"));
    assert!(result.to_uppercase().contains(".INT. GANESH'S HOUSE - DAY #10#"));
    assert!(result.contains("@REENA"));
    assert!(result.contains("@ROCKY"));
    assert!(result.contains("@SINDHU"));
    assert!(result.contains("!Reena closes the kitchen door and looks at rocky and sindhu"));
    assert!(result.contains("> CUT TO:"));
    assert!(!result.contains("(CONT'D)"));
    assert!(!result.contains("(MORE)"));
}

#[test]
fn test_a4_and_usletter_consistency() {
    let a4_path = "TestingFiles/sample_A4.pdf";
    let us_path = "TestingFiles/sample_USLETTER.pdf";
    if !Path::new(a4_path).exists() || !Path::new(us_path).exists() {
        return;
    }

    let a4 = parse_pdf_file_to_fountain(a4_path).unwrap();
    let us = parse_pdf_file_to_fountain(us_path).unwrap();

    assert_eq!(a4, us);
}
