import { CanActivateFn, Router } from '@angular/router';
import { inject } from '@angular/core';
import { ApiService } from '../services/api.service';

export const authGuard: CanActivateFn = (route, state) => {
  const apiService = inject(ApiService);
  const router = inject(Router);

  if (apiService.isLoggedIn()) {
    return true;
  } else {
    router.navigate(['/login']); // Redirige si no hay token
    return false;
  }
};
