import { startSignIn } from "../auth";
import { API_BASE } from "../api";

export default function SignIn() {
  return (
    <div className="min-h-screen flex items-center justify-center px-6 relative z-10">
      <div className="max-w-md w-full text-center animate-rise">
        <p className="font-mono text-xs tracking-[0.3em] text-ember uppercase mb-6">
          Aftercode · Studio
        </p>
        <h1 className="font-display text-5xl sm:text-6xl leading-[0.95] mb-5">
          Your coding
          <br />
          sessions,
          <span className="italic text-ember"> on air.</span>
        </h1>
        <p className="text-muted text-lg mb-10 leading-relaxed">
          A personal podcast of everything you and your agent built today —
          ready to listen, filter by topic, and learn from.
        </p>
        <button
          onClick={() => startSignIn(API_BASE)}
          className="font-sans font-semibold text-ink bg-ember hover:bg-emberdim transition-colors rounded-xl px-8 py-3.5 text-lg shadow-[0_8px_30px_rgba(224,121,75,0.25)]"
        >
          Sign in →
        </button>
        <p className="text-muted/70 text-xs mt-6 font-mono">
          opens {API_BASE || "this backend"}/cli/authorize
        </p>
      </div>
    </div>
  );
}
