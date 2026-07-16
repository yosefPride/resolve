import { useAuth } from '../hooks/useAuth';
import ProfileSummary from '../features/account/ProfileSummary';
import ProfileForm from '../features/account/ProfileForm';
import ChangePasswordForm from '../features/account/ChangePasswordForm';

export default function AccountPage() {
  const { user } = useAuth();

  return (
    <section className="mx-auto flex max-w-7xl flex-col gap-6 px-4 py-20 sm:px-6 lg:px-8">
      <h1 className="text-2xl font-bold text-white">Account</h1>
      <div className="grid gap-6 lg:grid-cols-2">
        <div className="flex flex-col gap-6">
          <ProfileSummary user={user} />
          <ProfileForm />
        </div>
        <ChangePasswordForm />
      </div>
    </section>
  );
}
