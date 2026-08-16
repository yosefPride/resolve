import { useAuth } from '../hooks/useAuth';
import ProfileSummary from '../features/account/ProfileSummary';
import ProfileForm from '../features/account/ProfileForm';
import ChangePasswordForm from '../features/account/ChangePasswordForm';

export default function AccountPage() {
  const { user } = useAuth();

  return (
    <section className="flex flex-col gap-6">
      <h1 className="text-2xl font-bold text-white">Account</h1>
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <div className="flex flex-col gap-6">
          <ProfileSummary user={user} />
          <ProfileForm />
        </div>
        <ChangePasswordForm />
      </div>
    </section>
  );
}
