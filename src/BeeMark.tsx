export default function BeeMark({ className = "" }: { className?: string }) {
  return <svg className={className} viewBox="0 0 32 32" aria-hidden="true" fill="none">
    <path d="M14 13C10 6.5 4.8 7 4.1 10.5C3.4 14 7.8 16.3 13 16L14 13Z" fill="currentColor" opacity=".52"/>
    <path d="M18 13C22 6.5 27.2 7 27.9 10.5C28.6 14 24.2 16.3 19 16L18 13Z" fill="currentColor" opacity=".52"/>
    <path d="M13.7 9.1C12.9 6.7 11.2 5.6 9.8 5.4M18.3 9.1C19.1 6.7 20.8 5.6 22.2 5.4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round"/>
    <path d="M16 7.5C11.9 7.5 9.5 11 9.5 16.2C9.5 21.5 12.2 25.2 16 27C19.8 25.2 22.5 21.5 22.5 16.2C22.5 11 20.1 7.5 16 7.5Z" fill="currentColor"/>
    <path d="M10.2 13.2C13.6 14.5 18.4 14.5 21.8 13.2M9.6 17.6C13.4 19 18.6 19 22.4 17.6M11 22C14.1 23 17.9 23 21 22" stroke="#17140D" strokeWidth="2.5"/>
    <ellipse cx="16" cy="9.8" rx="4.2" ry="3.1" fill="#17140D"/>
  </svg>;
}
