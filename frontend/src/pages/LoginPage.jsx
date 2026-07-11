import LoginForm from '../features/auth/LoginForm';

export default function LoginPage() {
  return (
    <section className="mx-auto flex max-w-md flex-col gap-6 px-4 py-20 sm:px-6 lg:px-8">
      <h1 className="text-center text-2xl font-bold text-white">Log in</h1>
      <LoginForm />
    </section>
  );
}
