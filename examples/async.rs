use indexmap::IndexMap;
use rootcause::{
    Report,
    handlers::{self, FormattingFunction},
    hooks::formatting_overrides::{AttachmentFormattingOverride, register_attachment_hook},
    prelude::ResultExt,
    report,
    report_attachment::ReportAttachmentRef,
    report_attachments::ReportAttachments,
};

fn from_err() -> Result<std::fs::File, std::io::Error> {
    std::fs::File::open("/notexist")
}

#[derive(Debug)]
struct Wat(&'static str);

impl core::fmt::Display for Wat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "I am something that requires\na lot of space to show\nwhat I have to say!\nThis is what I have to say though: {}",
            self.0
        )
    }
}

struct WatHandler;

impl AttachmentFormattingOverride<Wat> for WatHandler {
    fn preferred_formatting_style(
        &self,
        _attachment: ReportAttachmentRef<'_, dyn core::any::Any>,
        _report_formatting_function: FormattingFunction,
    ) -> handlers::AttachmentFormattingStyle {
        handlers::AttachmentFormattingStyle {
            // placement: handlers::AttachmentFormattingPlacement::Appendix {
            //     appendix_name: "wat-appendix",
            // },
            // placement: handlers::AttachmentFormattingPlacement::Inline,
            placement: handlers::AttachmentFormattingPlacement::InlineWithHeader { header: "Wat" },
            // placement: handlers::AttachmentFormattingPlacement::Opaque,
            function: handlers::FormattingFunction::Debug,
            priority: 0,
        }
    }
}

fn foo() -> Result<(), Report> {
    std::fs::read("/tmp/bla").attach("wat")?;
    Ok(())
}

fn bar() -> Result<(), Report> {
    foo().context("I am at bar").attach("With extra info")?;
    Ok(())
}

fn baz() -> Result<(), Report> {
    bar().context("At baz")?;
    Ok(())
}

// fn foo_sendsync() -> Result<(), ErrSync> {
//     Ok(())
// }
//
// fn foo_local() -> Result<(), ErrLocal> {
//     Ok(())
// }
//
// fn bar_sendsync() -> Result<(), Report<dyn Any, Mutable, Local>> {
//     foo_sendsync().into_report().attach?;
//     foo_local()?;
//     Ok(())
// }

#[tokio::main]
async fn main() {
    let task = tokio::task::spawn(wat());
    task.await.unwrap();
    // wat().await;
}

async fn wat() {
    register_attachment_hook(WatHandler);

    println!("{}", Some("wat").map(|v| report!("hello: {v}")).unwrap());
    // println!("{}", report!("hello"));
    println!("{}", baz().unwrap_err());
    // return;

    // print_types(report!("bla{}", x));
    // return;

    let mut x = IndexMap::<u8, u8>::new();
    x.entry(42).or_insert_with(|| {
        println!("{}", Some("foo").map(|_v| report!(":3")).unwrap());
        32
    });

    let report1: Report = report!(from_err().unwrap_err())
        .attach("This is\nblabla")
        .attach("Tried to open userconfig.toml")
        .context("Unable to parse user config")
        .attach("This is\nblabla")
        .attach(Wat("hello"))
        .attach("a string with a newline")
        .context("Unable to start up nuclear reactor\nbla")
        .into_dyn_any();
    // println!("{report1}");
    let report1 = report1.into_cloneable();

    let report2 = Report::from_parts::<handlers::Display>(
        "Parent",
        vec![report1.clone(), report1.clone()].into(),
        ReportAttachments::new(),
    );
    // println!("{report2}");

    let report2 = report2.into_cloneable().into_dyn_any();

    let report3 = Report::from_parts::<handlers::Display>(
        "level 3",
        vec![
            report2.clone(),
            report1.clone(),
            report2.clone(),
            report1.clone(),
        ]
        .into(),
        ReportAttachments::new(),
    );
    // println!("{report3}");

    let _ = (report1, report2, report3);
}
