import { usePaginatedServerTask } from '@minimact/core';

interface User {
  id: number;
  name: string;
  email: string;
  role: string;
}

interface UserFilters {
  role?: string;
  search?: string;
}

export function UserList() {
  // Paginated server task with intelligent prefetching
  const users = usePaginatedServerTask<User, UserFilters>(
    async ({ page, pageSize, filters }) => {
      // Server-side pagination query
      // This gets transpiled to C# and executed on the server
      const skip = (page - 1) * pageSize;
      const take = pageSize;

      // Simulate database query
      const allUsers = generateMockUsers(100);

      // Apply filters
      let filtered = allUsers;
      if (filters.role) {
        filtered = filtered.filter(u => u.role === filters.role);
      }
      if (filters.search) {
        const searchLower = filters.search.toLowerCase();
        filtered = filtered.filter(u =>
          u.name.toLowerCase().includes(searchLower) ||
          u.email.toLowerCase().includes(searchLower)
        );
      }

      // Return paginated results
      return filtered.slice(skip, skip + take);
    },
    {
      pageSize: 10,
      getTotalCount: async (filters) => {
        // Server-side count query
        const allUsers = generateMockUsers(100);

        let filtered = allUsers;
        if (filters.role) {
          filtered = filtered.filter(u => u.role === filters.role);
        }
        if (filters.search) {
          const searchLower = filters.search.toLowerCase();
          filtered = filtered.filter(u =>
            u.name.toLowerCase().includes(searchLower) ||
            u.email.toLowerCase().includes(searchLower)
          );
        }

        return filtered.length;
      },
      prefetchNext: true,  // Intelligent prefetching
      runtime: 'csharp'    // Can also be 'rust' for Rayon parallelism
    }
  );

  return (
    <div className="user-list">
      <h1>User Directory</h1>
      <p>Page {users.page} of {users.totalPages} ({users.total} total users)</p>

      {users.pending && <div className="loading">Loading...</div>}

      {users.error && <div className="error">Error: {users.error}</div>}

      <table>
        <thead>
          <tr>
            <th>ID</th>
            <th>Name</th>
            <th>Email</th>
            <th>Role</th>
          </tr>
        </thead>
        <tbody>
          {users.items.map(user => (
            <tr key={user.id}>
              <td>{user.id}</td>
              <td>{user.name}</td>
              <td>{user.email}</td>
              <td>{user.role}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <div className="pagination">
        <button
          onClick={() => users.prev()}
          disabled={!users.hasPrev}
        >
          Previous
        </button>

        <span>Page {users.page}</span>

        <button
          onClick={() => users.next()}
          disabled={!users.hasNext}
        >
          Next
        </button>

        <button onClick={() => users.goto(1)}>First</button>
        <button onClick={() => users.goto(users.totalPages)}>Last</button>
        <button onClick={() => users.refresh()}>Refresh</button>
      </div>
    </div>
  );
}

// Helper function to generate mock users
function generateMockUsers(count: number): User[] {
  const roles = ['Admin', 'User', 'Editor', 'Viewer'];
  const firstNames = ['Alice', 'Bob', 'Charlie', 'Diana', 'Eve', 'Frank', 'Grace', 'Henry'];
  const lastNames = ['Smith', 'Johnson', 'Williams', 'Brown', 'Jones', 'Garcia', 'Miller', 'Davis'];

  const users: User[] = [];
  for (let i = 1; i <= count; i++) {
    const firstName = firstNames[Math.floor(Math.random() * firstNames.length)];
    const lastName = lastNames[Math.floor(Math.random() * lastNames.length)];

    users.push({
      id: i,
      name: `${firstName} ${lastName}`,
      email: `${firstName.toLowerCase()}.${lastName.toLowerCase()}@example.com`,
      role: roles[Math.floor(Math.random() * roles.length)]
    });
  }

  return users;
}
