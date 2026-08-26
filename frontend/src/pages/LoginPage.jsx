import LoginForm from '../features/auth/LoginForm';

export default function LoginPage() {
  return (
    <div className="flex min-h-dvh items-center justify-center bg-black px-4 py-12 sm:px-6 lg:px-8">
      <section className="flex w-full max-w-md flex-col gap-6">
        <h1 className="text-center text-2xl font-bold text-white">Log in</h1>
        <LoginForm />
      </section>
    </div>
  );
}
