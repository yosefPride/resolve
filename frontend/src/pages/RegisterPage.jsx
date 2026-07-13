import RegisterForm from '../features/auth/RegisterForm';

export default function RegisterPage() {
  return (
    <section className="mx-auto flex max-w-md flex-col gap-6 px-4 py-20 sm:px-6 lg:px-8">
      <h1 className="text-center text-2xl font-bold text-white">Create your account</h1>
      <RegisterForm />
    </section>
  );
}
