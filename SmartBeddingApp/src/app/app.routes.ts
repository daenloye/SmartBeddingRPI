import { Routes } from '@angular/router';
import { LoginComponent } from './login/login.component';
import { TemplatePanelComponent } from './panel/template-panel/template-panel.component';
import { HomeComponent } from './panel/home/home.component';
import { ConnectivityComponent } from './panel/connectivity/connectivity.component';
import { StorageComponent } from './panel/storage/storage.component';
import { authGuard } from './guards/auth.guard';

export const routes: Routes = [
  {path: '', component: LoginComponent},
  {
    path:'panel',
    component: TemplatePanelComponent,
    canActivate: [authGuard],
    children:[
      {path: '', component: HomeComponent},
      {path: 'connectivity', component: ConnectivityComponent},
      {path: 'storage', component: StorageComponent},
    ]

  }
];
