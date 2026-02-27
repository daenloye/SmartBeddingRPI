// auth.guard.ts
import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { ApiService } from '../services/api.service';
import { catchError, map, of } from 'rxjs';

export const authGuard: CanActivateFn = (route, state) => {
  const apiService = inject(ApiService);
  const router = inject(Router);

  if (!apiService.isLoggedIn()) {
    router.navigate(['/']);
    return false;
  }

  // Si hay token, verificamos realmente con el servidor
  return apiService.verifyToken().pipe(
    map(res => {
      if (res.result) return true;
      router.navigate(['/']);
      return false;
    }),
    catchError(() => {
      apiService.handleUnauthorized();
      return of(false);
    })
  );
};
