import { ComponentFixture, TestBed } from '@angular/core/testing';

import { ConnectivityComponent } from './connectivity.component';

describe('ConnectivityComponent', () => {
  let component: ConnectivityComponent;
  let fixture: ComponentFixture<ConnectivityComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ConnectivityComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(ConnectivityComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
