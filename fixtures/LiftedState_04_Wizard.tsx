// Pattern 2.2: Parent Modifying Child State - Wizard Flow Control
// Parent orchestrates wizard workflow and controls step progression

import { Component, state, setState } from '@minimact/core';

// Child component: Step1
export function Step1() {
  const complete = state.complete;
  const data = state.data || {};

  const handleChange = (field: string, value: string) => {
    const newData = { ...data, [field]: value };
    setState('data', newData);

    // Auto-mark complete when all fields filled
    const isComplete = newData.name && newData.email;
    if (isComplete !== complete) {
      setState('complete', isComplete);
    }
  };

  return (
    <div className={`wizard-step ${complete ? 'complete' : ''}`}>
      <h2>Step 1: Basic Information</h2>
      <input
        id="step1-name"
        type="text"
        placeholder="Name"
        value={data.name || ''}
        onInput={(e) => handleChange('name', e.target.value)}
      />
      <input
        id="step1-email"
        type="email"
        placeholder="Email"
        value={data.email || ''}
        onInput={(e) => handleChange('email', e.target.value)}
      />
      {complete && <span className="check">✓ Complete</span>}
    </div>
  );
}

// Child component: Step2
export function Step2() {
  const complete = state.complete;
  const data = state.data || {};

  const handleChange = (field: string, value: string) => {
    const newData = { ...data, [field]: value };
    setState('data', newData);

    // Auto-mark complete when all fields filled
    const isComplete = newData.address && newData.city;
    if (isComplete !== complete) {
      setState('complete', isComplete);
    }
  };

  return (
    <div className={`wizard-step ${complete ? 'complete' : ''}`}>
      <h2>Step 2: Address</h2>
      <input
        id="step2-address"
        type="text"
        placeholder="Address"
        value={data.address || ''}
        onInput={(e) => handleChange('address', e.target.value)}
      />
      <input
        id="step2-city"
        type="text"
        placeholder="City"
        value={data.city || ''}
        onInput={(e) => handleChange('city', e.target.value)}
      />
      {complete && <span className="check">✓ Complete</span>}
    </div>
  );
}

// Child component: Step3
export function Step3() {
  const complete = state.complete;
  const data = state.data || {};

  const handleChange = (field: string, value: string) => {
    const newData = { ...data, [field]: value };
    setState('data', newData);

    // Auto-mark complete when all fields filled
    const isComplete = newData.cardNumber && newData.cvv;
    if (isComplete !== complete) {
      setState('complete', isComplete);
    }
  };

  return (
    <div className={`wizard-step ${complete ? 'complete' : ''}`}>
      <h2>Step 3: Payment</h2>
      <input
        id="step3-card"
        type="text"
        placeholder="Card Number"
        value={data.cardNumber || ''}
        onInput={(e) => handleChange('cardNumber', e.target.value)}
      />
      <input
        id="step3-cvv"
        type="text"
        placeholder="CVV"
        value={data.cvv || ''}
        onInput={(e) => handleChange('cvv', e.target.value)}
      />
      {complete && <span className="check">✓ Complete</span>}
    </div>
  );
}

// Parent component: WizardPage
export default function WizardPage() {
  const step1Complete = state["Step1.complete"];
  const step2Complete = state["Step2.complete"];
  const step3Complete = state["Step3.complete"];

  const currentStep = !step1Complete ? 1
                    : !step2Complete ? 2
                    : !step3Complete ? 3
                    : 4;

  const handleNext = () => {
    if (currentStep === 1 && !step1Complete) {
      // Force step 1 completion
      setState("Step1.complete", true);
    } else if (currentStep === 2 && !step2Complete) {
      setState("Step2.complete", true);
    } else if (currentStep === 3 && !step3Complete) {
      setState("Step3.complete", true);
    }
  };

  const handleBack = () => {
    if (currentStep === 2) {
      // Allow going back by marking previous step incomplete
      setState("Step1.complete", false);
    } else if (currentStep === 3) {
      setState("Step2.complete", false);
    } else if (currentStep === 4) {
      setState("Step3.complete", false);
    }
  };

  const handleComplete = () => {
    const allData = {
      step1: state["Step1.data"],
      step2: state["Step2.data"],
      step3: state["Step3.data"]
    };
    console.log('Wizard completed:', allData);
    alert('Wizard completed successfully!');
  };

  return (
    <div id="wizard-root">
      <h1>Setup Wizard</h1>

      {/* Progress indicator */}
      <div className="progress-bar">
        <div
          id="progress-1"
          className={`step ${step1Complete ? 'complete' : currentStep === 1 ? 'active' : ''}`}
        >
          1
        </div>
        <div
          id="progress-2"
          className={`step ${step2Complete ? 'complete' : currentStep === 2 ? 'active' : ''}`}
        >
          2
        </div>
        <div
          id="progress-3"
          className={`step ${step3Complete ? 'complete' : currentStep === 3 ? 'active' : ''}`}
        >
          3
        </div>
      </div>

      {/* Conditionally render steps */}
      {currentStep >= 1 && (
        <Component name="Step1" state={{ complete: false, data: {} }}>
          <Step1 />
        </Component>
      )}

      {currentStep >= 2 && (
        <Component name="Step2" state={{ complete: false, data: {} }}>
          <Step2 />
        </Component>
      )}

      {currentStep >= 3 && (
        <Component name="Step3" state={{ complete: false, data: {} }}>
          <Step3 />
        </Component>
      )}

      {/* Completion message */}
      {currentStep === 4 && (
        <div id="completion-message" className="completion">
          <h2>All Steps Complete!</h2>
          <p>You have completed all wizard steps.</p>
          <button
            id="complete-btn"
            type="button"
            onClick={handleComplete}
          >
            Finish
          </button>
        </div>
      )}

      {/* Navigation */}
      <div className="wizard-nav">
        <button
          id="back-btn"
          type="button"
          onClick={handleBack}
          disabled={currentStep === 1}
        >
          ← Back
        </button>
        <button
          id="next-btn"
          type="button"
          onClick={handleNext}
          disabled={currentStep === 4}
        >
          Next →
        </button>
      </div>

      {/* Status display for testing */}
      <div id="status" className="status">
        <p>Current Step: <span id="current-step">{currentStep}</span></p>
        <p>Step 1 Complete: <span id="step1-status">{step1Complete ? 'Yes' : 'No'}</span></p>
        <p>Step 2 Complete: <span id="step2-status">{step2Complete ? 'Yes' : 'No'}</span></p>
        <p>Step 3 Complete: <span id="step3-status">{step3Complete ? 'Yes' : 'No'}</span></p>
      </div>
    </div>
  );
}
