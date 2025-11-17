export function UserProfile({ user, loading }: { user: { name: string }, loading: boolean }) {
  return (
    <div>
      {loading ? <p>Loading...</p> : <p>Hello, {user.name}!</p>}
    </div>
  );
}
