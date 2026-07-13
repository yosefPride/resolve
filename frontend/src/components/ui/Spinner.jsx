export default function Spinner() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-black">
      <div className="h-10 w-10 animate-spin rounded-full border-2 border-white/10 border-t-white" />
    </div>
  );
}
