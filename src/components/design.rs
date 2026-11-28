use leptos::prelude::*;
use leptos_router::components::A;

pub const CARD: &str = "rounded-card border border-slate-800 bg-white shadow-card";
pub const CARD_PAD: &str = "px-[22px] pt-[18px] pb-5";
pub const CARD_HEAD: &str = "-mx-[22px] flex flex-wrap items-center justify-between gap-2 border-b border-slate-800 px-[22px] pb-3.5";
pub const SERIF_H1: &str = "font-serif text-[30px] font-semibold leading-tight text-slate-200";
pub const SERIF_H2: &str = "font-serif text-[19px] font-semibold text-slate-200";
pub const SERIF_PAGE_H2: &str = "font-serif text-[28px] font-semibold text-slate-200";
pub const BTN: &str = "inline-flex min-h-10 items-center justify-center gap-[7px] whitespace-nowrap rounded-full border border-slate-700 bg-white px-[18px] text-[14.5px] font-semibold leading-none text-slate-200 hover:bg-sand-100 disabled:cursor-not-allowed disabled:opacity-55";
pub const BTN_SM: &str = "inline-flex min-h-[34px] items-center justify-center gap-[7px] whitespace-nowrap rounded-full border border-slate-700 bg-white px-3.5 text-[13.5px] font-semibold leading-none text-slate-200 hover:bg-sand-100 disabled:cursor-not-allowed disabled:opacity-55";
pub const BTN_PRIMARY: &str = "inline-flex min-h-10 items-center justify-center gap-[7px] whitespace-nowrap rounded-full border border-primary-600 bg-primary-600 px-[18px] text-[14.5px] font-semibold leading-none text-white hover:bg-primary-700 disabled:cursor-not-allowed disabled:opacity-55";
pub const BTN_GREEN: &str = "inline-flex min-h-10 items-center justify-center gap-[7px] whitespace-nowrap rounded-full border border-ok-700 bg-ok-700 px-[18px] text-[14.5px] font-semibold leading-none text-white hover:bg-ok-800 disabled:cursor-not-allowed disabled:opacity-55";
pub const LBL: &str = "text-[13.5px] font-bold text-ink-600";
pub const HINT: &str = "text-[12.5px] text-slate-500";
pub const MUTED: &str = "text-slate-500";
pub const FIELD: &str = "flex min-w-0 flex-col gap-1.5";
pub const INPUT: &str = "min-h-[42px] w-full rounded-xl border border-sand-500 bg-white px-3.5 py-2 text-[14.5px] text-slate-200 focus:border-primary-600 focus:outline-none";
pub const INPUT_READONLY: &str = "min-h-[42px] w-full rounded-xl border border-slate-800 bg-sand-200 px-3.5 py-2 text-[14.5px] text-ink-600";
pub const ERRMSG: &str = "flex items-start gap-1.5 text-[13.5px] font-semibold text-danger-700";
pub const KICKER: &str = "text-[12.5px] font-bold uppercase tracking-[.08em] text-slate-500";
pub const SAVED: &str = "inline-flex items-center gap-1.5 text-[13.5px] font-semibold text-ok-700";
pub const TD: &str = "border-b border-sand-300 px-3.5 py-3 align-top group-last:border-b-0";
pub const LINK: &str =
    "font-semibold text-primary-300 underline-offset-[3px] hover:text-primary-800";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Check,
    Warning,
    Cross,
    Lock,
    Plus,
    ArrowLeft,
    ArrowRight,
    Pencil,
}

pub fn icon(kind: Icon, class: &'static str) -> AnyView {
    let path = match kind {
        Icon::Check => view! { <path d="M5 12l5 5L20 7"></path> }.into_any(),
        Icon::Warning => view! { <path d="M12 3l9.5 17h-19z"></path><path d="M12 10v4M12 17.5v.01"></path> }.into_any(),
        Icon::Cross => view! { <path d="M6 6l12 12M18 6L6 18"></path> }.into_any(),
        Icon::Lock => view! { <rect x="5" y="11" width="14" height="10" rx="2"></rect><path d="M8 11V7a4 4 0 0 1 8 0v4"></path> }.into_any(),
        Icon::Plus => view! { <path d="M12 5v14M5 12h14"></path> }.into_any(),
        Icon::ArrowLeft => view! { <path d="M19 12H5"></path><path d="M11 18l-6-6 6-6"></path> }.into_any(),
        Icon::ArrowRight => view! { <path d="M5 12h14"></path><path d="M13 6l6 6-6 6"></path> }.into_any(),
        Icon::Pencil => view! { <path d="M4 20h4L19 9l-4-4L4 16z"></path><path d="M14 6l4 4"></path> }.into_any(),
    };
    view! {
        <svg
            class=class
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            {path}
        </svg>
    }
    .into_any()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Green,
    Yellow,
    Red,
    Sand,
    Dark,
}

impl Tone {
    pub fn classes(self) -> &'static str {
        match self {
            Tone::Green => "bg-ok-50 text-ok-700",
            Tone::Yellow => "bg-primary-100 text-caution-800",
            Tone::Red => "bg-danger-100 text-danger-700",
            Tone::Sand => "bg-sand-300 text-slate-400",
            Tone::Dark => "bg-slate-200 text-white",
        }
    }
}

#[component]
pub fn StatusPill(
    #[prop(into)] label: String,
    tone: Tone,
    #[prop(optional)] icon_kind: Option<Icon>,
    #[prop(optional)] small: bool,
) -> impl IntoView {
    let size = if small {
        "px-2 text-[12px] leading-5"
    } else {
        "px-[11px] py-[3px] text-[13px] leading-5"
    };
    let class = format!(
        "inline-flex items-center gap-1.5 whitespace-nowrap rounded-full font-semibold {size} {}",
        tone.classes()
    );
    view! {
        <span class=class>
            {icon_kind.map(|kind| icon(kind, "h-3.5 w-3.5 flex-none"))}
            {label}
        </span>
    }
}

#[component]
pub fn VolunteersOnlyBadge() -> impl IntoView {
    view! {
        <span class="inline-flex items-center gap-[5px] whitespace-nowrap rounded-full border border-caution-300 bg-caution-50 px-2.5 py-0.5 text-[12.5px] font-bold text-caution-800">
            {icon(Icon::Lock, "h-3.5 w-3.5 flex-none")}
            "Volunteers only"
        </span>
    }
}

#[component]
pub fn DarkBanner(children: Children) -> impl IntoView {
    view! {
        <div role="note" class="flex items-center gap-2.5 rounded-[14px] bg-slate-200 px-[18px] py-3 text-[14.5px] font-bold text-white">
            {icon(Icon::Lock, "h-4 w-4 flex-none")}
            <span>{children()}</span>
        </div>
    }
}

#[component]
pub fn PageHeader(
    #[prop(into)] back_href: String,
    #[prop(into)] back_label: String,
    children: Children,
    #[prop(optional)] actions: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="flex flex-wrap items-end justify-between gap-x-5 gap-y-3">
            <div class="min-w-0">
                <A
                    href=back_href
                    attr:class="inline-flex min-h-8 items-center gap-1.5 text-[13.5px] font-semibold text-slate-400 no-underline hover:text-slate-200"
                >
                    {icon(Icon::ArrowLeft, "h-4 w-4 flex-none")}
                    {back_label}
                </A>
                <div class="mt-1">{children()}</div>
            </div>
            {actions.map(|actions| view! { <div class="flex flex-wrap items-center gap-2">{actions()}</div> })}
        </div>
    }
}

#[component]
pub fn SectionCard(
    #[prop(into)] title: String,
    #[prop(optional, into)] anchor: Option<String>,
    #[prop(optional, into)] number: Option<String>,
    #[prop(optional)] required: Option<bool>,
    children: Children,
) -> impl IntoView {
    view! {
        <section id=anchor class=format!("{CARD} {CARD_PAD} scroll-mt-4")>
            <div class=CARD_HEAD>
                <h2 class=format!("{SERIF_H2} flex items-baseline gap-2")>
                    {number.map(|n| view! { <span class="text-[13px] font-bold uppercase text-primary-700">{n}</span> })}
                    {title}
                </h2>
                <div class="flex items-center gap-2">
                    {required.map(|required| {
                        if required {
                            view! { <StatusPill label="Required" tone=Tone::Red small=true /> }.into_any()
                        } else {
                            view! { <StatusPill label="Optional" tone=Tone::Sand small=true /> }.into_any()
                        }
                    })}
                </div>
            </div>
            <div class="mt-3">{children()}</div>
        </section>
    }
}

#[component]
pub fn EdgeTable(
    headers: Vec<(&'static str, Option<&'static str>)>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="-mx-[22px] overflow-x-auto">
            <table class="w-full border-collapse text-[14.5px]">
                <thead>
                    <tr>
                        {headers
                            .into_iter()
                            .map(|(label, width)| view! {
                                <th
                                    scope="col"
                                    style=width.map(|w| format!("width: {w}"))
                                    class="whitespace-nowrap border-b border-slate-800 bg-sand-50 px-3.5 py-2.5 text-left text-[12.5px] font-bold uppercase tracking-[.05em] text-slate-500 first:pl-[22px] last:pr-[22px]"
                                >
                                    {label}
                                </th>
                            })
                            .collect_view()}
                    </tr>
                </thead>
                <tbody class="[&_td:first-child]:pl-[22px] [&_td:last-child]:pr-[22px] [&_tr:last-child_td]:border-b-0">
                    {children()}
                </tbody>
            </table>
        </div>
    }
}
