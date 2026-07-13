import type { Metadata } from "next";
import { headers } from "next/headers";
import { Archivo, Plus_Jakarta_Sans, Roboto_Mono } from "next/font/google";
import "./globals.css";

const archivo = Archivo({ variable: "--font-archivo", subsets: ["latin"] });
const jakarta = Plus_Jakarta_Sans({ variable: "--font-jakarta", subsets: ["latin"] });
const mono = Roboto_Mono({ variable: "--font-mono", subsets: ["latin"] });

export async function generateMetadata(): Promise<Metadata> {
  const requestHeaders = await headers();
  const host = requestHeaders.get("x-forwarded-host") ?? requestHeaders.get("host") ?? "localhost:3001";
  const protocol = requestHeaders.get("x-forwarded-proto") ?? (host.startsWith("localhost") ? "http" : "https");
  const socialImage = `${protocol}://${host}/og.png`;
  return {
    title: "BridgeVoice: Vibe Code With Your Voice | 99+ Languages",
    description: "Privacy-first desktop dictation for prompts, code, commits, and docs — on-device or cloud, across macOS, Windows, and Linux.",
    icons: { icon: "/images/bridgevoice-icon.svg" },
    openGraph: { title: "BridgeVoice — Vibe code at the speed of thought.", description: "Hold a key, speak, release. Your words land wherever the cursor blinks.", images: [socialImage] },
    twitter: { card: "summary_large_image", title: "BridgeVoice — Vibe code at the speed of thought.", description: "Privacy-first voice dictation for people who build software.", images: [socialImage] },
  };
}

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return <html lang="en"><body className={`${archivo.variable} ${jakarta.variable} ${mono.variable}`}>{children}</body></html>;
}
