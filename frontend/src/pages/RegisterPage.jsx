import RegisterForm from '../features/auth/RegisterForm';

export default function RegisterPage() {
  return (
    <div className="flex min-h-dvh items-center justify-center bg-black px-4 py-12 sm:px-6 lg:px-8">
      <section className="flex w-full max-w-md flex-col gap-6">
        <h1 className="text-center text-2xl font-bold text-white">Create your account</h1>
        <RegisterForm />
      </section>
    </div>
  );
}
