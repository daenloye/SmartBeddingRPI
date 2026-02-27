import { ComponentFixture, TestBed } from '@angular/core/testing';

import { TemplatePanelComponent } from './template-panel.component';

describe('TemplatePanelComponent', () => {
  let component: TemplatePanelComponent;
  let fixture: ComponentFixture<TemplatePanelComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [TemplatePanelComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(TemplatePanelComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
