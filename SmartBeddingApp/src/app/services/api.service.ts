import { ApiLogin } from './../interfaces/api-login';
import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { Observable, tap } from 'rxjs';
import { ApiResponse } from '../interfaces/api-response';

@Injectable({
  providedIn: 'root'
})
export class ApiService {
  private readonly API_URL = 'http://192.168.0.112:8080';

  constructor(private http: HttpClient) {}

  // Handshake: Enviamos el código quemado, recibimos el token
  login(code: number): Observable<ApiResponse<string>> {
    //Generamos la estructura
    const payload:ApiLogin = { code };

    return this.http.post<ApiResponse<string>>(`${this.API_URL}/auth`, payload).pipe(
      tap(res => {
        console.log('Login response:', res);
        if (res.result && res.data) {
          localStorage.setItem('access_token', res.data);
        }
      })
    );
  }

  getToken(): string | null {
    return localStorage.getItem('access_token');
  }

  isLoggedIn(): boolean {
    return !!this.getToken();
  }

  logout() {
    localStorage.removeItem('access_token');
  }
}
