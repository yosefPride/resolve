import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../../hooks/useAuth';
import { useSubmit } from '../../hooks/useSubmit';
import Button from '../../components/ui/Button';
import Field from '../../components/ui/Field';
import Input from '../../components/ui/Input';

export default function RegisterForm() {
  const { register } = useAuth();
  const navigate = useNavigate();
  const [form, setForm] = useState({ name: '', email: '', password: '' });
  const { error, isPending, submit } = useSubmit(async () => {
    await register(form);
    navigate('/dashboard');
  }, 'Registration failed. Please try again.');

  function handleChange(event) {
    const { name, value } = event.target;
    setForm((prev) => ({ ...prev, [name]: value }));
  }

  return (
    <form onSubmit={submit} className="flex flex-col gap-4">
      <Field label="Name">
        <Input type="text" name="name" value={form.name} onChange={handleChange} required />
      </Field>

      <Field label="Email">
        <Input type="email" name="email" value={form.email} onChange={handleChange} required />
      </Field>

      <Field label="Password">
        <Input
          type="password"
          name="password"
          value={form.password}
          onChange={handleChange}
          required
        />
      </Field>

      {error && <p className="text-sm text-red-500">{error}</p>}

      <Button type="submit" disabled={isPending} className="mt-2">
        {isPending ? 'Creating account…' : 'Sign up'}
      </Button>
    </form>
  );
}
