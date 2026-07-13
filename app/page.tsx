"use client";

import { FormEvent, useState } from "react";
import Image from "next/image";

const steps = [
  { name: "Hold", color: "blue", copy: "One global hotkey, system-wide — Fn on the Mac. Hold to record, or flip to toggle mode and dictate hands-free." },
  { name: "Speak", color: "green", copy: "Say it like you would to a teammate — agent prompts, commit messages, docs, Slack replies." },
  { name: "Release", color: "yellow", copy: "Transcription runs where you chose: on-device through whisper.cpp or Parakeet, or in the cloud on Whisper Large-v3-Turbo." },
  { name: "Land", color: "violet", copy: "Text lands wherever the cursor blinks — editor, terminal, browser, chat — in under a second." },
];

const specs = [
  ["Local or cloud — your call", "On-device whisper.cpp or Parakeet keeps audio on your machine. Cloud mode runs Groq Whisper Large-v3-Turbo — 99+ languages, auto-detected."],
  ["Seven local models", "Six Whisper checkpoints from Tiny (75 MB) to Large-v3 (3.1 GB), plus Parakeet V3 for multilingual — fully offline. Pick the size your hardware likes."],
  ["Native, not Electron", "Tauri 2 and Rust, with a lock-free realtime audio pipeline. Recording starts in under 10ms; text lands in under a second."],
  ["Lives in the menu bar", "Launches at login and waits in the tray. Signed, notarized macOS builds; updates ship themselves."],
];

const products = [
  ["BridgeSpace", "The desktop where agents work — 16 parallel terminals and a kanban that dispatches agents.", "#6e8eff"],
  ["BridgeAgent", "Hand it the mission — it designs, ships, and fixes software in a loop. Beta.", "#79d6af"],
  ["BridgeMCP", "Shared memory and task routing for your agents — one MCP endpoint for Cursor, Claude Code, Windsurf, and Codex.", "#f0cf43"],
  ["BridgeSwarm", "Multi-agent teams inside BridgeSpace — lead, explore, ship, and review in parallel.", "#a884ff"],
  ["BridgeShot", "The native macOS screenshot tool your Mac deserves.", "#ef8e74"],
];

const faqs = [
  ["Is my voice data actually private?", "In local mode, yes — audio is processed on-device by whisper.cpp or Parakeet and never leaves your machine. Cloud mode encrypts audio over HTTPS to the BridgeMind API. You choose."],
  ["Can I dictate hands-free?", "Yes. Push-to-talk records while the hotkey is held; toggle mode starts on one press and stops on the next. Both shortcuts are configurable in Dashboard → Shortcuts."],
  ["Which models are available?", "Seven local models: six Whisper checkpoints from Tiny (75 MB) to Large-v3 (3.1 GB), plus NVIDIA Parakeet V3 for multilingual. Cloud mode runs Groq Whisper Large-v3-Turbo with 99+ languages."],
  ["What platforms does BridgeVoice run on?", "macOS, Windows, and Linux, built with Tauri 2 and Rust. macOS builds are code-signed and notarized, and the app updates itself."],
  ["How do I get BridgeVoice?", "BridgeVoice is included with the Pro and Ultra plans — the same subscription that unlocks BridgeSpace, BridgeMCP, BridgeMemory, and BridgeShot. Download it, sign in, and dictate."],
];

const footerColumns = {
  Ecosystem: ["BridgeMCP", "BridgeSpace", "BridgeVoice", "BridgeShot"],
  Explore: ["Docs", "Open Source", "Blog", "Careers", "Merch"],
  Learn: ["Vibe Coding", "Agentic Coding", "AI Coding", "Pricing"],
  Company: ["About Us", "Changelog", "Roadmap", "Media Assets", "Contact Us"],
  Community: ["Discord", "Events", "ViewCreator", "Affiliate Program", "Bug Bounty"],
};

function Brand() {
  return <a className="brand" href="#top" aria-label="BridgeMind home"><span className="brand-mark">ϟ</span><span>BridgeMind</span></a>;
}

export default function Home() {
  const [subscribed, setSubscribed] = useState(false);
  const submit = (event: FormEvent<HTMLFormElement>) => { event.preventDefault(); setSubscribed(true); };

  return (
    <div id="top">
      <a className="skip" href="#main-content">Skip to main content</a>
      <header className="site-header">
        <Brand />
        <nav className="desktop-nav" aria-label="Main navigation">
          <a href="#pricing">Pricing</a>
          <details><summary>Ecosystem</summary><div className="nav-popover"><a href="#ecosystem">All products</a><a href="#how-it-works">How it works</a><a href="#under-hood">Technology</a></div></details>
          <details><summary>Community</summary><div className="nav-popover"><a href="https://discord.gg/bridgemind">Discord</a><a href="#footer">Newsletter</a></div></details>
          <details><summary>Company</summary><div className="nav-popover"><a href="#faq">FAQ</a><a href="#footer">Contact</a></div></details>
        </nav>
        <div className="header-actions"><a className="login" href="https://www.bridgemind.ai/login">Log In</a><a className="button light small" href="https://www.bridgemind.ai/signup">Get Started</a></div>
        <details className="mobile-menu"><summary aria-label="Open navigation"><span></span><span></span></summary><nav><a href="#download">Download</a><a href="#how-it-works">How it works</a><a href="#under-hood">Technology</a><a href="#faq">FAQ</a></nav></details>
      </header>

      <main id="main-content">
        <section className="hero panel" id="download">
          <div className="ambient ambient-one" />
          <div className="shell">
            <nav className="breadcrumb" aria-label="Breadcrumb"><a href="#top">Home</a><span>/</span><span>BridgeVoice</span></nav>
            <div className="product-kicker"><span className="app-icon-wrap"><Image src="/images/bridgevoice-icon.svg" alt="BridgeVoice application icon" width={30} height={30} priority /></span><strong>BridgeVoice</strong><span className="live"><i /> Live</span></div>
            <h1><span className="sr-only">BridgeVoice — </span>Vibe code at the<br className="desktop-break" /> speed of thought.</h1>
            <p className="hero-copy">Hold a key, speak, release — your words land wherever the cursor blinks, in any app. Private on-device Whisper or cloud in 99+ languages — your call.</p>
            <div className="downloads">
              <a className="button light download-main" href="https://downloads.bridgemind.ai/bridgevoice/latest/macos/BridgeVoice.dmg?v=2"><span className="apple">●</span> Download for macOS <em>.dmg</em></a>
              <span className="available">Also available for</span>
              <div className="alt-downloads"><a href="https://downloads.bridgemind.ai/bridgevoice/latest/windows/BridgeVoice-setup.exe?v=2">⊞&nbsp; Windows</a><a href="https://downloads.bridgemind.ai/bridgevoice/latest/linux/BridgeVoice.AppImage?v=2">◩&nbsp; Linux</a></div>
              <small>Requires macOS 12+ (Apple Silicon & Intel)<br />✓ Included with BridgeMind Pro&nbsp;&nbsp; ⟲ Need an older version?</small>
            </div>
            <p className="tech-line">macOS · Windows · Linux · Built with Tauri 2 + Rust</p>
            <div className="media-frame hero-media"><span className="frame-label">BridgeVoice in action</span><video controls muted loop playsInline preload="metadata" poster="/media/bridgevoice-promo.webp" aria-label="BridgeVoice demo"><source src="https://downloads.bridgemind.ai/videos/marketing/bridgevoice-promo-zoom-tracking.mp4" type="video/mp4" /></video></div>
          </div>
        </section>

        <section className="section panel deep" id="how-it-works" aria-labelledby="steps-title"><div className="shell"><h2 id="steps-title">Hold. Speak. Release. Land.</h2><p className="section-intro">The whole interface is one key. No window to focus, no app to switch — dictation follows your cursor across the desktop.</p><ol className="steps">{steps.map((step) => <li key={step.name}><h3><span className={`dot ${step.color}`} />{step.name}</h3><p>{step.copy}</p></li>)}</ol></div></section>

        <section className="section panel" aria-labelledby="film-title"><div className="shell"><h2 id="film-title">Thirty seconds, sound on.</h2><p className="section-intro">The BridgeVoice demo film — hold, speak, release, and the words land while you watch.</p><div className="media-frame film-media"><span className="frame-label">BridgeVoice in 30 seconds</span><video controls playsInline preload="none" poster="/media/bridgevoice-film.webp" aria-label="BridgeVoice demo film"><source src="https://downloads.bridgemind.ai/videos/marketing/bridgevoice-marketing-video.mp4" type="video/mp4" /></video></div></div></section>

        <section className="section panel deep" id="under-hood" aria-labelledby="under-title"><div className="shell"><h2 id="under-title">Under the hood.</h2><dl className="spec-list">{specs.map(([title, copy]) => <div key={title}><dt>{title}</dt><dd>{copy}</dd></div>)}</dl></div></section>

        <section className="section panel" id="ecosystem" aria-labelledby="ecosystem-title"><div className="shell"><div className="section-heading-row"><div><h2 id="ecosystem-title">One subscription.<br />The whole ecosystem.</h2><p className="section-intro">One plan unlocks every tool you need to vibe code — this one and everything below.</p></div><a className="text-link" href="https://www.bridgemind.ai/pricing">Compare plans ↗</a></div><div className="product-grid">{products.map(([name, copy, color], index) => <a className={`product-card card-${index + 1}`} href="#pricing" key={name} style={{ "--accent": color } as React.CSSProperties}><span className="card-number">0{index + 1}</span><span className="mini-mark">ϟ</span><h3>{name}</h3><p>{copy}</p><span className="card-arrow">↗</span></a>)}</div></div></section>

        <section className="section panel deep faq-section" id="faq" aria-labelledby="faq-title"><div className="shell narrow"><h2 id="faq-title">Frequently asked.</h2><div className="faq-list">{faqs.map(([q, a]) => <details key={q}><summary><span>{q}</span><i>+</i></summary><p>{a}</p></details>)}</div></div></section>

        <section className="cta panel" id="pricing"><div className="ambient ambient-two" /><div className="shell narrow"><span className="eyebrow">Your voice. Your code. Your machine.</span><h2>Stop typing.<br />Start building.</h2><p>BridgeVoice is included with BridgeMind Pro and Ultra. Download it, sign in, and your cursor starts listening.</p><a className="button light" href="https://downloads.bridgemind.ai/bridgevoice/latest/macos/BridgeVoice.dmg?v=2">Download BridgeVoice&nbsp; ↓</a></div></section>
      </main>

      <footer id="footer"><div className="shell"><div className="footer-top"><div className="footer-about"><Brand /><p>The hub of the vibe coding space. Ship software at the speed of thought alongside autonomous AI teammates.</p><div className="socials"><a href="https://x.com/bridgemindai">X</a><a href="https://www.youtube.com/@bridgemindai">YouTube</a><a href="https://discord.gg/bridgemind">Discord</a></div></div><div className="newsletter"><h3>Subscribe &amp; Get 50% Off</h3>{subscribed ? <p className="success">You&apos;re on the list — welcome aboard.</p> : <form onSubmit={submit}><label className="sr-only" htmlFor="email">Email address</label><input id="email" required type="email" placeholder="Enter your email" /><button aria-label="Subscribe to newsletter">→</button></form>}<small>No spam / Unsubscribe anytime / 50% off your first 3 months</small></div></div><nav className="footer-links" aria-label="Footer">{Object.entries(footerColumns).map(([heading, links]) => <div key={heading}><h3>{heading}</h3>{links.map(link => <a href="#top" key={link}>{link}</a>)}</div>)}</nav><div className="legal"><span>© 2026 BridgeMind AI. All rights reserved.</span><div><a href="#top">Privacy Policy</a><a href="#top">Terms of Service</a><a href="#top">Status</a><a href="#top">Sitemap</a></div></div></div></footer>
    </div>
  );
}
